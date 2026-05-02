use lazy_static::lazy_static;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Deserialize, Clone)]
pub struct ModelConfig {
    pub hidden_size: usize,
    pub seq_len: usize,
    pub text_vocab_size: usize,
    pub vision_vocab_size: usize,
    pub audio_vocab_size: usize,
    pub qkv_size: usize,
    pub ffn_size: usize,
    pub kv_offset: usize,
    pub v_offset: usize,
    pub head_dim: usize,
    pub num_heads: usize,
    pub num_kv_heads: usize,
    pub num_layers: usize,
    pub num_experts: usize,
}

lazy_static! {
    pub static ref LILITH_CONFIG: ModelConfig = {
        let mut file = File::open("config.json").unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        serde_json::from_str(&contents).unwrap()
    };
}
