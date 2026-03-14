import os
import json
import asyncio
from groq import Groq
from src.utils import logger

class PredictorAgent:
    def __init__(self):
        # We will simulate an ensemble by calling multiple different models on Groq
        # as a stand-in for OpenAI/Anthropic/Deepseek due to key availability
        self.client = Groq(api_key=os.getenv("GROQ_API_KEY"))
        
        # We'll use an ensemble of smaller models and roles to create a Mixture of Experts
        self.ensemble = [
            {
                "role": "Primary Forecaster",
                "system_prompt": "You are the Primary Forecaster. Analyze all data impartially and provide the most likely real-world probability of the event occurring.",
                "model": "llama-3.3-70b-versatile",
                "weight": 0.30
            },
            {
                "role": "News Analyst",
                "system_prompt": "You are the News Analyst. Weigh recent news phrasing, public sentiment, and media narrative heavily when calculating probability.",
                "model": "mixtral-8x7b-32768",
                "weight": 0.20
            },
            {
                "role": "Bull Advocate",
                "system_prompt": "You are the Bull Advocate. Look for EVERY reason why this market will resolve YES. Argue the upside and provide a highly optimistic probability.",
                "model": "llama-3.1-8b-instant",
                "weight": 0.20
            },
            {
                "role": "Bear Advocate",
                "system_prompt": "You are the Bear Advocate. Look for EVERY reason why this market will resolve NO. Argue the downside and provide a highly pessimistic, low probability.",
                "model": "llama-3.1-8b-instant",
                "weight": 0.15
            },
            {
                "role": "Risk Manager",
                "system_prompt": "You are the Risk Manager. Evaluate edge cases, black swans, counter-party risks, and regulatory hurdles. Be highly skeptical when generating the probability.",
                "model": "llama-3.3-70b-versatile",
                "weight": 0.15
            }
        ]

    async def _predict_single(self, agent_config, market_title, current_price, research_json):
        """Fetches a prediction from a single agent based on their specific role."""
        model_name = agent_config["model"]
        role = agent_config["role"]
        custom_system = agent_config["system_prompt"]
        
        system_prompt = (
            f"{custom_system}\n\n"
            "Your job is to read a research brief and the current market odds, "
            "and output a JSON with your predicted true probability based on your role.\n\n"
            "OUTPUT FORMAT:\n"
            "{\n"
            '  "p_model": 0.52,\n'
            '  "reasoning": "Confidence is high based on..."\n'
            "}\n"
            "p_model must be a float between 0.00 and 1.00."
        )
        
        user_payload = f"""
        Market: {market_title}
        Current Market Implied Probability (p_market): {current_price}
        
        --- RESEARCH BRIEF ---
        {research_json}
        """
        
        try:
            # We use synchronous calls here wrapped in to_thread, or run them sequentially 
            # for the sake of simplicity in this script. Groq is fast.
            response = self.client.chat.completions.create(
                model=model_name,
                max_tokens=500,
                response_format={"type": "json_object"},
                messages=[
                    {"role": "system", "content": system_prompt},
                    {"role": "user", "content": user_payload}
                ]
            )
            
            text = response.choices[0].message.content
            if "```json" in text:
                text = text.split("```json\n")[1].split("\n```")[0]
            elif "```" in text:
                text = text.split("```\n")[1].split("\n```")[0]
                
            return json.loads(text)
        except Exception as e:
            logger.error(f"[{role} - {model_name}] Prediction failed: {e}")
            return {"p_model": current_price, "reasoning": "Failed to predict.", "weight": agent_config["weight"], "role": role}
            
        return {"p_model": json.loads(text).get("p_model", current_price), "weight": agent_config["weight"], "role": role}

    async def evaluate_edge(self, market_title, current_price, research_json):
        """Runs the ensemble evaluation to calculate the edge."""
        logger.info(f"Starting Ensemble Prediction for: {market_title}")
        
        # Run all highly-specialized agent roles concurrently
        tasks = [self._predict_single(config, market_title, current_price, research_json) for config in self.ensemble]
        results = await asyncio.gather(*tasks)
        
        # Calculate Weighted Consensus
        weighted_sum = 0.0
        total_weight_used = 0.0
        
        for res in results:
            p_model = res.get("p_model")
            weight = res.get("weight", 0.0)
            
            if isinstance(p_model, (int, float)):
                weighted_sum += p_model * weight
                total_weight_used += weight
                
        if total_weight_used == 0:
            consensus_p = current_price
        else:
            # Normalize to account for any models that failed and dropped their weight chunks
            consensus_p = weighted_sum / total_weight_used
            
        edge = consensus_p - current_price
        signal = "TRADE" if edge > 0.04 else "WAIT"
        
        return {
            "market_id": market_title,
            "p_market": current_price,
            "p_model": round(consensus_p, 4),
            "edge": round(edge, 4),
            "signal": signal,
            "models_polled": len(valid_probs),
            "reasoning": f"Consensus reached across {len(valid_probs)} models."
        }

if __name__ == "__main__":
    from dotenv import load_dotenv
    load_dotenv()
    
    # Mock data
    market = "MicroStrategy sells any Bitcoin"
    price = 0.45
    research = json.dumps({
      "market_id": "MSTRBTC",
      "narrative_consensus": "mixed",
      "key_facts": [
        "Chairman denies selling Bitcoin, says MicroStrategy will buy more.",
        "CEO says Bitcoin sales are possible, but no confirmation on sale."
      ],
      "disconfirming_evidence": "CEO's possible intentions vs Chairman's denial.",
      "sentiment_gap": "Market uncertainty is high."
    })
    
    agent = PredictorAgent()
    result = asyncio.run(agent.evaluate_edge(market, price, research))
    print(json.dumps(result, indent=2))
