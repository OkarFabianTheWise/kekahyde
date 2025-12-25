use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::{AddBos, LlamaModel, Special, params::LlamaModelParams};
use llama_cpp_2::sampling::LlamaSampler;
use num_cpus;

use std::num::NonZeroU32;
use std::sync::Arc;

pub struct Model {
    backend: Arc<LlamaBackend>,
    pub model: Option<Arc<LlamaModel>>,
}

impl Model {
    pub fn new() -> Result<Self, String> {
        let backend = LlamaBackend::init().map_err(|e| format!("Backend init failed: {e:?}"))?;

        Ok(Self {
            backend: Arc::new(backend),
            model: None,
        })
    }

    pub async fn load_model(&mut self, path: &str) -> Result<(), String> {
        let params = LlamaModelParams::default();

        let model = LlamaModel::load_from_file(&self.backend, path, &params)
            .map_err(|e| format!("Model load failed: {e:?}"))?;

        self.model = Some(Arc::new(model));
        Ok(())
    }

    pub fn is_loaded(&self) -> bool {
        self.model.is_some()
    }

    pub async fn run_prompt(&self, prompt: &str) -> Result<String, String> {
        let model = self.model.as_ref().ok_or("Model not loaded")?;

        let threads = num_cpus::get();

        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(2048))
            .with_n_threads(threads as i32);

        let mut ctx = model
            .new_context(&self.backend, ctx_params)
            .map_err(|e| format!("Context creation failed: {e:?}"))?;

        ctx.clear_kv_cache();

        // ✅ Qwen2.5 uses ChatML format
        let formatted_prompt = format!(
            "<|im_start|>system\nYou are a helpful assistant.<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            prompt
        );

        // Tokenize with BOS
        let tokens = model
            .str_to_token(&formatted_prompt, AddBos::Always)
            .map_err(|e| format!("Tokenize failed: {e:?}"))?;

        println!("Prompt tokens: {}", tokens.len());

        // Evaluate prompt
        let mut batch = LlamaBatch::new(2048, 1);
        batch
            .add_sequence(&tokens, 0, true)
            .map_err(|e| format!("Add sequence failed: {e:?}"))?;

        ctx.decode(&mut batch)
            .map_err(|e| format!("Eval failed: {e:?}"))?;

        // ✅ Better sampling parameters for Qwen2.5
        let mut sampler = LlamaSampler::chain_simple(vec![
            LlamaSampler::temp(0.7),
            LlamaSampler::top_k(40),
            LlamaSampler::top_p(0.9, 1),
            LlamaSampler::min_p(0.05, 1),
            LlamaSampler::dist(42),
        ]);

        let mut output = String::new();
        let mut pos = tokens.len() as i32;
        let mut logits_index = (tokens.len() - 1) as i32;

        let max_tokens = 256;
        let mut generated = 0;

        // Get stop tokens
        let eos_token = model.token_eos();

        loop {
            if generated >= max_tokens {
                break;
            }

            let token = sampler.sample(&ctx, logits_index);

            // Check for end tokens
            if token == eos_token {
                break;
            }

            // ✅ Also check for <|im_end|> token (Qwen's stop token)
            let text = model
                .token_to_str(token, Special::Tokenize)
                .map_err(|e| format!("Decode failed: {e:?}"))?;

            // Stop if we hit the end marker
            if text.contains("<|im_end|>") {
                break;
            }

            output.push_str(&text);

            // Check context limit
            if pos >= 2047 {
                break;
            }

            // Prepare next token
            let mut batch = LlamaBatch::new(2048, 1);
            batch
                .add(token, pos, &[0], true)
                .map_err(|e| format!("Add token failed: {e:?}"))?;
            pos += 1;

            ctx.decode(&mut batch)
                .map_err(|e| format!("Eval token failed: {e:?}"))?;

            logits_index = 0;
            generated += 1;
        }

        Ok(output.trim().to_string())
    }
}
