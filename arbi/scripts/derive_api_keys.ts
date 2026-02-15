import { ClobClient } from "@polymarket/clob-client";
import { Wallet } from "@ethersproject/wallet";
import * as dotenv from "dotenv";

dotenv.config();

const privateKey = process.env.PRIVATE_KEY;
if (!privateKey) {
    console.error("Please set PRIVATE_KEY in .env file");
    process.exit(1);
}

const host = 'https://clob.polymarket.com';
const signer = new Wallet(privateKey);
const clobClient = new ClobClient(host, 137, signer);

(async () => {
    try {
        console.log("Deriving API Key...");
        const apiKey = await clobClient.deriveApiKey();
        console.log("API Key Credentials:");
        console.log(JSON.stringify(apiKey, null, 2));
        console.log("\nAdd these to your .env file as:");
        console.log(`POLY_API_KEY=${apiKey.apiKey}`);
        console.log(`POLY_API_SECRET=${apiKey.secret}`);
        console.log(`POLY_PASSPHRASE=${apiKey.passphrase}`);
    } catch (error) {
        console.error("Error deriving API key:", error);
    }
})();
