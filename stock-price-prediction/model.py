import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression 
from sklearn.model_selection import train_test_split
from sklearn.metrics import accuracy_score
from sklearn.preprocessing import StandardScaler
from sklearn.pipeline import Pipeline
from skl2onnx import convert_sklearn
from skl2onnx.common.data_types import FloatTensorType
import optuna
import os
import warnings

warnings.filterwarnings("always")

np.random.seed(42)

class StockSentimentModel:
    def __init__(self):
        self.best_C = 0.0

    def extract_features(self, df: pd.DataFrame):
        features = pd.DataFrame()
        features['price_change'] = df['close'].pct_change()
        features['price_momentum'] = df['close'].pct_change().rolling(5).mean()
        features['price_volatility'] = df['close'].pct_change().rolling(10).std()
        features['volume_momentum'] = df['volume'].pct_change().rolling(5).mean()
        features['volume_ratio'] = df['volume'] / df['volume'].rolling(10).mean()
        features['price_acceleration'] = features['price_change'].diff()
        features = features.dropna()

        return features
    
    def prepare_labels(self, df: pd.DataFrame, forward_window: int = 5) -> np.ndarray:
        future_returns = df['price'].shift(-forward_window) / df['price'] - 1
        labels = (future_returns > 0).astype(int)
        return labels[:-forward_window]
    
    def train(self, features: pd.DataFrame, labels: np.ndarray, best_C):
        self.model = LogisticRegression(C=best_C, solver="liblinear", intercept_scaling=1.2)
        scaler = StandardScaler()
        X_scaled = scaler.fit_transform(features)
        self.model.fit(X_scaled, labels)
        
        print(f"Model accuracy: {self.model.score(X_scaled, labels):.4f}")

    def evaluate(self, X_test: np.ndarray, y_test: np.ndarray) -> np.ndarray:
        probs = self.model.predict_proba(X_test)
        y_pred = self.model.predict(X_test)
        accuracy = accuracy_score(y_test, y_pred)
        print(f"Test Accuracy: {accuracy:.4f}")
        return probs

    def export_to_onnx(self, feature_count: int) -> None:
        model_dir = "../model"

        os.makedirs(model_dir, exist_ok=True)

        path = os.path.join(model_dir, "MarketRegimeModel.onnx")

        initial_type = [('float_input', FloatTensorType([None, feature_count]))]
        options = {id(self.model): {'zipmap': False, 'raw_scores': True}}

        onx = convert_sklearn(self.model, initial_types=initial_type, options=options, target_opset=13)

        with open(path, "wb") as f:
            f.write(onx.SerializeToString())

def hyper_tune(X_train, X_test, y_train, y_test):
    def objective(trial):
        C = trial.suggest_loguniform('C', 1e-10, 1e10)
        model = Pipeline([
            ('scaler', StandardScaler()),
            ('logistic', LogisticRegression(C=C))
        ])
        model = model.fit(X_train, y_train)
        return model.score(X_test, y_test)

    study = optuna.create_study(direction='maximize')
    study.optimize(objective, n_trials=100)
    return study.best_params['C']

def main():
    df = pd.read_csv("TATAMOTORS_minute.csv")
    df['price'] = (df['open'] + df['close']) / 2

    df = df.iloc[-75000:]

    forward_window = 5

    model = StockSentimentModel()
    X = model.extract_features(df)

    df = df[-X.shape[0]:]
    Y = model.prepare_labels(df)
    X = X.iloc[:-forward_window]

    X_train, X_test, y_train, y_test = train_test_split(X, Y, random_state=420)

    best_C = hyper_tune(X_train, X_test, y_train, y_test)

    model.train(X_train, y_train, best_C=best_C)

    model.evaluate(X_test, y_test)

    model.export_to_onnx(X.shape[1])

if __name__ == "__main__":
    main()
