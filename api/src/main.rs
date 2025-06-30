use dotenv::dotenv;
use poem::{
    handler, listener::TcpListener, web::Json, Route, Server, IntoResponse, http::StatusCode
};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signer, Signature},
    system_instruction,
};
use spl_token::instruction as token_instruction;
use std::env;
use std::str::FromStr;
use base58::{ToBase58, FromBase58};
use base64::{Engine as _, engine::general_purpose};

#[derive(Serialize)]
struct ApiResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

fn success(data: serde_json::Value) -> (StatusCode, Json<ApiResponse>) {
    (
        StatusCode::OK,
        Json(ApiResponse { success: true, data: Some(data), error: None })
    )
}

fn error(msg: &str) -> (StatusCode, Json<ApiResponse>) {
    (
        StatusCode::BAD_REQUEST,
        Json(ApiResponse { success: false, data: None, error: Some(msg.to_string()) })
    )
}

// --- Endpoint Structs ---

#[derive(Serialize)]
struct KeypairResponse {
    pubkey: String,
    secret: String,
}

#[derive(Deserialize)]
struct CreateTokenRequest {
    #[serde(rename = "mintAuthority")]
    mint_authority: String,
    mint: String,
    decimals: u8,
}

#[derive(Serialize)]
struct AccountMetaCamel {
    pubkey: String,
    #[serde(rename = "isSigner")]
    is_signer: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "isWritable")]
    is_writable: Option<bool>,
}

#[derive(Serialize)]
struct InstructionResponseCreateToken {
    program_id: String,
    accounts: serde_json::Map<String, serde_json::Value>,
    instruction_data: String,
}

#[derive(Serialize)]
struct InstructionResponseMintToken {
    program_id: String,
    accounts: Vec<AccountMetaCamel>,
    instruction_data: String,
}

#[derive(Serialize)]
struct InstructionResponseSendSol {
    program_id: String,
    accounts: Vec<String>,
    instruction_data: String,
}

#[derive(Serialize)]
struct InstructionResponseSendToken {
    program_id: String,
    accounts: Vec<AccountMetaCamel>,
    instruction_data: String,
}

#[derive(Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

#[derive(Deserialize)]
struct SignMessageRequest {
    message: String,
    secret: String,
}

#[derive(Serialize)]
struct SignMessageResponse {
    signature: String,
    public_key: String,
    message: String,
}

#[derive(Deserialize)]
struct VerifyMessageRequest {
    message: String,
    signature: String,
    pubkey: String,
}

#[derive(Serialize)]
struct VerifyMessageResponse {
    valid: bool,
    message: String,
    pubkey: String,
}

#[derive(Deserialize)]
struct SendSolRequest {
    from: String,
    to: String,
    lamports: u64,
}

#[derive(Deserialize)]
struct SendTokenRequest {
    destination: String,
    mint: String,
    owner: String,
    amount: u64,
}

// --- Endpoints ---

#[handler]
async fn generate_keypair() -> (StatusCode, Json<ApiResponse>) {
    let keypair = Keypair::new();
    let resp = KeypairResponse {
        pubkey: keypair.pubkey().to_string(),
        secret: keypair.to_bytes().as_ref().to_base58(),
    };
    match serde_json::to_value(resp) {
        Ok(val) => success(val),
        Err(_) => error("Serialization error"),
    }
}

#[handler]
async fn create_token(Json(req): Json<CreateTokenRequest>) -> (StatusCode, Json<ApiResponse>) {
    let mint_authority = Pubkey::from_str(&req.mint_authority);
    let mint = Pubkey::from_str(&req.mint);
    if mint_authority.is_err() || mint.is_err() {
        return error("Invalid public key(s)");
    }
    let instruction = token_instruction::initialize_mint(
        &spl_token::id(),
        &mint.unwrap(),
        &mint_authority.unwrap(),
        None,
        req.decimals,
    );
    match instruction {
        Ok(ix) => {
            let mut accounts_map = serde_json::Map::new();
            for meta in ix.accounts.iter() {
                let mut obj = serde_json::Map::new();
                obj.insert("pubkey".to_string(), serde_json::Value::String(meta.pubkey.to_string()));
                obj.insert("isSigner".to_string(), serde_json::Value::Bool(meta.is_signer));
                obj.insert("isWritable".to_string(), serde_json::Value::Bool(meta.is_writable));
                accounts_map.insert(meta.pubkey.to_string(), serde_json::Value::Object(obj));
            }
            let resp = InstructionResponseCreateToken {
                program_id: ix.program_id.to_string(),
                accounts: accounts_map,
                instruction_data: general_purpose::STANDARD.encode(&ix.data),
            };
            match serde_json::to_value(resp) {
                Ok(val) => success(val),
                Err(_) => error("Serialization error"),
            }
        }
        Err(e) => error(&format!("Failed to create instruction: {e}")),
    }
}

#[handler]
async fn mint_token(Json(req): Json<MintTokenRequest>) -> (StatusCode, Json<ApiResponse>) {
    let mint = Pubkey::from_str(&req.mint);
    let destination = Pubkey::from_str(&req.destination);
    let authority = Pubkey::from_str(&req.authority);
    if mint.is_err() || destination.is_err() || authority.is_err() {
        return error("Invalid public key(s)");
    }
    let instruction = token_instruction::mint_to(
        &spl_token::id(),
        &mint.unwrap(),
        &destination.unwrap(),
        &authority.unwrap(),
        &[],
        req.amount,
    );
    match instruction {
        Ok(ix) => {
            let accounts = ix.accounts.iter().map(|meta| AccountMetaCamel {
                pubkey: meta.pubkey.to_string(),
                is_signer: meta.is_signer,
                is_writable: Some(meta.is_writable),
            }).collect();
            let resp = InstructionResponseMintToken {
                program_id: ix.program_id.to_string(),
                accounts,
                instruction_data: general_purpose::STANDARD.encode(&ix.data),
            };
            match serde_json::to_value(resp) {
                Ok(val) => success(val),
                Err(_) => error("Serialization error"),
            }
        }
        Err(e) => error(&format!("Failed to create instruction: {e}")),
    }
}

#[handler]
async fn sign_message(Json(req): Json<SignMessageRequest>) -> (StatusCode, Json<ApiResponse>) {
    if req.message.is_empty() || req.secret.is_empty() {
        return error("Missing required fields");
    }
    let secret_bytes = req.secret.from_base58();
    if let Ok(bytes) = secret_bytes {
        if let Ok(keypair) = Keypair::from_bytes(&bytes) {
            let signature = keypair.sign_message(req.message.as_bytes());
            let resp = SignMessageResponse {
                signature: general_purpose::STANDARD.encode(signature.as_ref()),
                public_key: keypair.pubkey().to_string(),
                message: req.message,
            };
            return match serde_json::to_value(resp) {
                Ok(val) => success(val),
                Err(_) => error("Serialization error"),
            };
        }
    }
    error("Invalid secret key")
}

#[handler]
async fn verify_message(Json(req): Json<VerifyMessageRequest>) -> (StatusCode, Json<ApiResponse>) {
    if req.message.is_empty() || req.signature.is_empty() || req.pubkey.is_empty() {
        return error("Missing required fields");
    }
    let pubkey = Pubkey::from_str(&req.pubkey);
    let signature_bytes = general_purpose::STANDARD.decode(&req.signature);
    if let (Ok(pubkey), Ok(sig_bytes)) = (pubkey, signature_bytes) {
        let signature = Signature::new(&sig_bytes);
        let valid = signature.verify(&pubkey.to_bytes(), req.message.as_bytes());
        let resp = VerifyMessageResponse {
            valid,
            message: req.message,
            pubkey: req.pubkey,
        };
        return match serde_json::to_value(resp) {
            Ok(val) => success(val),
            Err(_) => error("Serialization error"),
        };
    }
    error("Invalid signature or public key")
}

#[handler]
async fn send_sol(Json(req): Json<SendSolRequest>) -> (StatusCode, Json<ApiResponse>) {
    let from = Pubkey::from_str(&req.from);
    let to = Pubkey::from_str(&req.to);
    if from.is_err() || to.is_err() {
        return error("Invalid public key(s)");
    }
    if req.lamports == 0 {
        return error("Amount must be greater than zero");
    }
    let ix = system_instruction::transfer(&from.unwrap(), &to.unwrap(), req.lamports);
    let accounts = ix.accounts.iter().map(|meta| meta.pubkey.to_string()).collect();
    let resp = InstructionResponseSendSol {
        program_id: ix.program_id.to_string(),
        accounts,
        instruction_data: general_purpose::STANDARD.encode(&ix.data),
    };
    match serde_json::to_value(resp) {
        Ok(val) => success(val),
        Err(_) => error("Serialization error"),
    }
}

#[handler]
async fn send_token(Json(req): Json<SendTokenRequest>) -> (StatusCode, Json<ApiResponse>) {
    let destination = Pubkey::from_str(&req.destination);
    let mint = Pubkey::from_str(&req.mint);
    let owner = Pubkey::from_str(&req.owner);
    if destination.is_err() || mint.is_err() || owner.is_err() {
        return error("Invalid public key(s)");
    }
    if req.amount == 0 {
        return error("Amount must be greater than zero");
    }
    let destination = destination.unwrap();
    let ix = token_instruction::transfer(
        &spl_token::id(),
        &destination,
        &destination, // source and destination are the same for simplicity
        &owner.unwrap(),
        &[],
        req.amount,
    );
    match ix {
        Ok(ix) => {
            let accounts = ix.accounts.iter().map(|meta| AccountMetaCamel {
                pubkey: meta.pubkey.to_string(),
                is_signer: meta.is_signer,
                is_writable: None,
            }).collect();
            let resp = InstructionResponseSendToken {
                program_id: ix.program_id.to_string(),
                accounts,
                instruction_data: general_purpose::STANDARD.encode(&ix.data),
            };
            match serde_json::to_value(resp) {
                Ok(val) => success(val),
                Err(_) => error("Serialization error"),
            }
        }
        Err(e) => error(&format!("Failed to create instruction: {e}")),
    }
}

#[handler]
async fn health() -> (StatusCode, Json<ApiResponse>) {
    success(serde_json::json!({"status": "OK"}))
}

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    dotenv().ok();
    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let app = Route::new()
        .at("/health", health)
        .at("/keypair", generate_keypair)
        .at("/token/create", create_token)
        .at("/token/mint", mint_token)
        .at("/message/sign", sign_message)
        .at("/message/verify", verify_message)
        .at("/send/sol", send_sol)
        .at("/send/token", send_token);
    println!("🚀 Solana HTTP Server starting on {}", addr);
    Server::new(TcpListener::bind(addr))
        .run(app)
        .await
}
