use crate::bindings::exports::asterai::llm::llm::Guest;

#[allow(warnings)]
mod bindings {
    wit_bindgen::generate!({
        path: "wit/package.wasm",
        world: "component",
        generate_all,
    });
}

mod utils;
mod anthropic;
mod deepseek;
mod fireworks;
mod google;
mod groq;
mod mistral;
mod openai;
mod openrouter;
mod perplexity;
mod together;
mod venice;
mod xai;

struct Component;

impl Guest for Component {
    fn prompt(prompt: String, model: String) -> String {
        let Some((provider, model_name)) = model.split_once('/') else {
            return format!("error: invalid model format '{model}', expected 'provider/model'");
        };
        match provider {
            "openai" => openai::prompt(&prompt, model_name),
            "anthropic" => anthropic::prompt(&prompt, model_name),
            "mistral" => mistral::prompt(&prompt, model_name),
            "groq" => groq::prompt(&prompt, model_name),
            "google" => google::prompt(&prompt, model_name),
            "venice" => venice::prompt(&prompt, model_name),
            "xai" => xai::prompt(&prompt, model_name),
            "deepseek" => deepseek::prompt(&prompt, model_name),
            "together" => together::prompt(&prompt, model_name),
            "fireworks" => fireworks::prompt(&prompt, model_name),
            "perplexity" => perplexity::prompt(&prompt, model_name),
            "openrouter" => openrouter::prompt(&prompt, model_name),
            _ => format!("error: unsupported provider '{provider}'"),
        }
    }
}

bindings::export!(Component with_types_in bindings);
