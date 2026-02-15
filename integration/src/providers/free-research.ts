/**
 * Free Research Provider using Tavily (Search) + Groq (Reasoning)
 * Provides a $0 alternative to Perplexity
 */

import axios from "axios";
import type { PerplexityResponse, PerplexitySearchResult } from "./perplexity.js";
import { generateResearchQueries } from "./perplexity.js";

/**
 * Search the web using Tavily
 */
export async function searchTavily(query: string, apiKey: string): Promise<string> {
    try {
        const response = await axios.post("https://api.tavily.com/search", {
            api_key: apiKey,
            query: query,
            search_depth: "basic",
            include_answer: true,
            max_results: 5
        });

        // Tavily often provides a direct 'answer' or we can join the 'results'
        return response.data.answer || response.data.results.map((r: any) => r.content).join("\n\n");
    } catch (err: any) {
        if (err.response) {
            console.error("Tavily Search Error Details:", JSON.stringify(err.response.data, null, 2));
        }
        console.error("Tavily Search Error:", err.message);
        throw err;
    }
}

/**
 * Reason over search results using Groq
 */
export async function reasonWithGroq(
    query: string,
    context: string,
    apiKey: string,
    model: string = "llama-3.3-70b-versatile"
): Promise<string> {
    try {
        const response = await axios.post("https://api.groq.com/openai/v1/chat/completions", {
            model,
            messages: [
                {
                    role: "system",
                    content: "You are a research assistant focused on prediction markets. Analyze the provided search context and answer the user's query concisely. Focus on facts and probabilities."
                },
                {
                    role: "user",
                    content: `Search Results Context:\n${context}\n\nUser Query: ${query}`
                }
            ],
            temperature: 0.2,
            max_tokens: 1024
        }, {
            headers: {
                "Authorization": `Bearer ${apiKey}`,
                "Content-Type": "application/json"
            }
        });

        return response.data.choices[0].message.content;
    } catch (err) {
        console.error("Groq Reasoning Error:", err);
        throw err;
    }
}

/**
 * Checks if two markets (e.g., Polymarket and Kalshi) are equivalent
 */
export async function checkMarketEquivalence(
    marketA: { title: string, description?: string },
    marketB: { title: string, subtitle?: string },
    groqApiKey: string
): Promise<{
    isEquivalent: boolean,
    score: number,
    reasoning: string
}> {
    try {
        const response = await axios.post("https://api.groq.com/openai/v1/chat/completions", {
            model: "llama-3.3-70b-versatile",
            messages: [
                {
                    role: "system",
                    content: "You are an expert in prediction markets. Compare two market listings and determine if they cover the exact same event and outcome. Output as a JSON object with 'score' (0.0 to 1.0), 'isEquivalent' (boolean, true if score >= 0.85), and 'reasoning' (brief sentence)."
                },
                {
                    role: "user",
                    content: `Market A (Source): ${marketA.title} ${marketA.description || ""}\nMarket B (Target): ${marketB.title} ${marketB.subtitle || ""}`
                }
            ],
            response_format: { type: "json_object" },
            temperature: 0.1
        }, {
            headers: {
                "Authorization": `Bearer ${groqApiKey}`,
                "Content-Type": "application/json"
            }
        });

        return JSON.parse(response.data.choices[0].message.content);
    } catch (err) {
        console.error("Equivalence Check Error:", err);
        return {
            isEquivalent: false,
            score: 0,
            reasoning: "Failed to perform AI equivalence check."
        };
    }
}

/**
 * Run a full research cycle using Tavily + Groq
 */
export async function runFreeResearch(
    marketTitle: string,
    tavilyKey: string,
    groqKey: string,
    category?: string,
    customQueries?: string[]
): Promise<PerplexitySearchResult> {
    const queries = customQueries || generateResearchQueries(marketTitle, category);
    const results: PerplexityResponse[] = [];

    for (const query of queries) {
        try {
            const searchContext = await searchTavily(query, tavilyKey);
            const answer = await reasonWithGroq(query, searchContext, groqKey);

            results.push({
                query,
                answer,
                citations: [] // Tavily citations could be mapped here if needed
            });
        } catch (err) {
            results.push({
                query,
                answer: "",
                citations: [],
                error: err instanceof Error ? err.message : String(err)
            });
        }

        // Slight delay for rate limits
        await new Promise(resolve => setTimeout(resolve, 500));
    }

    return {
        queries,
        results
    };
}
