use crate::bindings::asterai::llm::llm;

const MODEL: &str = "anthropic/claude-sonnet-4-5";
const SYSTEM_PROMPT: &str = "\
You are a weather notification assistant for Wagga Wagga, Australia. \
Your focus is on notifying about lightning. \
Format the given weather data into a brief, natural SMS message. \
Keep it concise (1-3 sentences). No hashtags, no emojis. \
Just relay the information naturally as if texting a friend.";

pub fn rewrite(raw_message: &str) -> String {
    let prompt = format!("{SYSTEM_PROMPT}\n\n{raw_message}");
    let response = llm::prompt(&prompt, MODEL);
    if response.is_empty() {
        raw_message.to_string()
    } else {
        response
    }
}
