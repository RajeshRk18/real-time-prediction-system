use prost::Message as ProstMessage;
use reqwest::Client;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{Duration, sleep};
//use chrono::{Local, NaiveTime, Weekday};
use bytes::BytesMut;
use futures_util::{SinkExt, stream::StreamExt};
use log::{error, info};
use serde::{Deserialize, Serialize};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use anyhow::Result;
use crate::config::WebSocketConfig;
use crate::error::DataIngestionError;
use crate::pb::feed::FeedUnion;
use crate::pb::full_feed::FullFeedUnion;

#[derive(Serialize)]
pub struct AccessTokenRequest<'a> {
    client_id: &'a str,
}

#[derive(Serialize)]
pub struct MarketFeedSubscribeMessage {
    guid: String,
    method: String,
    data: RequestData,
}

#[derive(Serialize)]
pub struct RequestData {
    mode: String,
    instrument_keys: Vec<String>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct InputData {
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub vol: i64,
}

pub struct StockDataStream {
    config: WebSocketConfig,
    // producer handle
    kafka_handler: mpsc::Sender<InputData>,
}

impl StockDataStream {
    pub fn new(
        kafka_handler: mpsc::Sender<InputData>,
        config: WebSocketConfig,
    ) -> Self {
        Self {
            config,
            kafka_handler,
        }
    }

    pub async fn authorize(&mut self) -> Result<()> {
        // Get authoried url to fetch data
        let get_auth_redirect_url = "https://api.upstox.com/v3/feed/market-data-feed/authorize";
        let mut bearer_token = String::from("Bearer ");
        bearer_token.push_str(&self.config.access_token);

        let client = Client::new();

        let response = client
            .get(get_auth_redirect_url)
            .header("Authorization", bearer_token)
            .header("Accept", "application/json")
            .send()
            .await?;

        let json_resp: serde_json::Value = response.json().await?;
        println!("{:?}", json_resp);
        let redirect_uri = json_resp["data"]["authorized_redirect_uri"]
            .as_str()
            .expect("Redirect URI Not found");

        info!("Obtained authorized redirect URI: {:?}", redirect_uri);

        let (ws_stream, _) = connect_async(redirect_uri).await?;
        info!("Connecting to Upstox Websocket...");

        let (mut write, mut read) = ws_stream.split();

        sleep(Duration::from_secs(1)).await;

        let mut ins_keys = Vec::new();
        ins_keys.push(self.config.instrument_key.clone());

        let req_data = RequestData {
            mode: "full".to_string(),
            instrument_keys: ins_keys,
        };

        let sub_message = MarketFeedSubscribeMessage {
            guid: "someguid".to_string(),
            method: "sub".to_string(),
            data: req_data,
        };

        let sub_json = serde_json::to_string(&sub_message)?;
        let sub_bytes = sub_json.into_bytes();
        write.send(Message::Binary(sub_bytes.into())).await?;

        info!("Subscribe message sent to get live data.");

        while let Some(message) = read.next().await {
            match message {
                Ok(Message::Binary(data)) => {
                    // Decode the binary data into a FeedResponse.
                    // Create a BytesMut buffer from the received data.
                    let mut buf = BytesMut::from(&data[..]);
                    match crate::pb::FeedResponse::decode(&mut buf) {
                        Ok(feed_response) => {
                            let feeds = extract_ohlc(feed_response.clone());
                            for feed in feeds.into_iter() {
                                self.kafka_handler.send(feed).await?;
                            }
                            println!("Received FeedResponse: {:#?}", feed_response);
                        }
                        Err(e) => {
                            error!("Cannot decode the protobuf response");
                        }
                    }
                }
                Ok(Message::Text(text)) => {
                    println!("Received text message: {}", text);
                }
                Ok(other) => {
                    println!("Received other message: {:?}", other);
                }
                Err(e) => {
                    eprintln!("Error reading message: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn reauthorize(&mut self) -> Result<(), DataIngestionError> {
        // LOGIN
        let auth_url = format!(
            "{:?}?client_id={:?}&redirect_uri={:?}&response_type=code",
            self.config.user_auth_url.clone(),
            self.config.api_key.clone(),
            self.config.redirect_uri.clone()
        );

        // Get code to generate token
        let resp = Client::new().get(auth_url).send().await?;

        let resp_json: serde_json::Value = resp
            .json()
            .await
            .map_err(DataIngestionError::ReqwestError)?;

        let code = resp_json["code"].as_str().expect("Cannot extract code");
        // Generate Access token
        let token_url = "https://api.upstox.com/v2/login/authorization/token";

        let client = Client::new();

        let mut params = HashMap::new();
        params.insert("code", code.to_owned());
        params.insert("client_id", self.config.api_key.clone());
        params.insert("client_secret", self.config.api_secret_key.clone());
        params.insert("redirect_uri", self.config.redirect_uri.clone());
        params.insert("grant_type", "authorization_code".to_string());

        let response = client
            .post(token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;

        let mut access_token = String::new();
        if response.status().is_success() {
            let json_response: serde_json::Value = response.json().await?;
            access_token = json_response["access_token"]
                .as_str()
                .expect("Cannot extract Access Token")
                .to_owned();
        } else {
            let error_text = response.text().await?;
            eprintln!("Error: {}", error_text);
        }

        let req_token_url = "https://api.upstox.com/v3/login/auth/token/request/:client_id";
        let client = Client::new();
        let param = AccessTokenRequest {
            client_id: &self.config.api_secret_key,
        };
        let response = client
            .post(req_token_url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&param)
            .send()
            .await?;

        Ok(())
    }

}

// extract the required data
fn extract_ohlc(feed_response: crate::pb::FeedResponse) -> Vec<InputData> {
    let mut feeds = Vec::new();
    for (key, feed) in feed_response.feeds.iter() {
        // Check if the feed contains a feed_union.
        if let Some(feed_union) = &feed.feed_union {
            // We expect the full feed case to contain OHLC data.
            if let FeedUnion::FullFeed(full_feed) = feed_union {
                if let Some(full_feed_union) = &full_feed.full_feed_union {
                    match full_feed_union {
                        FullFeedUnion::MarketFf(market_ff) => {
                            // Check if MarketFullFeed contains market_ohlc.
                            if let Some(market_ohlc) = &market_ff.market_ohlc {
                                println!("Feed key: {}", key);
                                // Iterate over the OHLC entries.
                                for ohlc in market_ohlc.ohlc.iter() {
                                    let feed = InputData {
                                        open: ohlc.open,
                                        high: ohlc.high,
                                        low: ohlc.low,
                                        close: ohlc.close,
                                        vol: ohlc.vol,
                                    };
                                    feeds.push(feed);
                                }
                                return feeds;
                            } else {
                                println!("No MarketOHLC data found in feed key: {}", key);
                            }
                        }
                        FullFeedUnion::IndexFf(_index_ff) => {
                            // This feed is for index data and does not contain OHLC.
                            println!(
                                "Feed key: {} is an Index feed. Skipping OHLC extraction.",
                                key
                            );
                        }
                    }
                } else {
                    println!("Feed key: {} FullFeedUnion is missing", key);
                }
            } else {
                println!("Feed key: {} is not a FullFeed (not market data)", key);
            }
        } else {
            println!("Feed key: {} has no feed_union", key);
        }
    }
    return feeds;
}
