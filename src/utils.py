import os
import logging
from dotenv import load_dotenv

def setup_env_and_logging(logger_name="predict_bot"):
    """
    Loads environment variables from .env and configures the centralized logger.
    """
    load_dotenv()
    
    # Ensure logs dir exists
    os.makedirs("logs", exist_ok=True)
    
    logger = logging.getLogger(logger_name)
    logger.setLevel(logging.INFO)
    
    # Prevent duplicate handlers if called multiple times in notebooks/interactive sessions
    if not logger.handlers:
        formatter = logging.Formatter("%(asctime)s [%(levelname)s] %(name)s: %(message)s")
        
        # Console handler
        ch = logging.StreamHandler()
        ch.setFormatter(formatter)
        logger.addHandler(ch)
        
        # File handler
        fh = logging.FileHandler("logs/bot.log")
        fh.setFormatter(formatter)
        logger.addHandler(fh)
        
    return logger

# Create a default logger instance
logger = setup_env_and_logging()
