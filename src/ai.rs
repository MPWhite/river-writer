use chrono::{DateTime, Local, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use crate::config::Config;
use reqwest::blocking::Client;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
pub struct PromptCache {
    #[serde(with = "chrono::serde::ts_seconds")]
    generated_at: DateTime<Utc>,
    prompts: HashMap<String, DailyPrompt>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DailyPrompt {
    pub prompt: String,
    pub theme: String,
    pub context: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Vec<Content>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Content {
    text: String,
}

pub struct PromptGenerator {
    api_key: String,
    cache_path: PathBuf,
    notes_dir: PathBuf,
}

impl PromptGenerator {
    pub fn new(config: &Config) -> Result<Self, Box<dyn Error>> {
        // Get API key from environment variable
        let api_key = std::env::var("ANTHROPIC_API_KEY")
            .map_err(|_| "ANTHROPIC_API_KEY environment variable not set")?;
        
        let cache_path = Self::get_cache_path(config);
        let notes_dir = PathBuf::from(&config.daily_notes_dir);
        
        Ok(PromptGenerator {
            api_key,
            cache_path,
            notes_dir,
        })
    }
    
    fn get_cache_path(_config: &Config) -> PathBuf {
        let mut path = dirs::config_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push("river");
        path.push("prompt_cache.json");
        path
    }
    
    pub fn load_cached_prompt(&self, date: &NaiveDate) -> Option<DailyPrompt> {
        // Try to load from cache
        if let Ok(contents) = fs::read_to_string(&self.cache_path) {
            if let Ok(cache) = serde_json::from_str::<PromptCache>(&contents) {
                // Check if cache is less than 7 days old
                let age = Utc::now().signed_duration_since(cache.generated_at);
                if age.num_days() < 7 {
                    let date_str = date.format("%Y-%m-%d").to_string();
                    return cache.prompts.get(&date_str).cloned();
                }
            }
        }
        None
    }
    
    pub fn generate_prompts(&self) -> Result<(), Box<dyn Error>> {
        println!("Analyzing recent notes...");
        
        // Collect recent notes (last 30 days)
        let recent_notes = self.collect_recent_notes(30)?;
        
        if recent_notes.is_empty() {
            println!("No recent notes found. Using default prompts.");
            return Ok(());
        }
        
        println!("Found {} recent notes. Generating personalized prompts...", recent_notes.len());
        
        // Analyze notes and generate prompts
        let prompts = self.analyze_and_generate(recent_notes)?;
        
        // Save to cache
        let cache = PromptCache {
            generated_at: Utc::now(),
            prompts,
        };
        
        // Ensure directory exists
        if let Some(parent) = self.cache_path.parent() {
            fs::create_dir_all(parent)?;
        }
        
        let json = serde_json::to_string_pretty(&cache)?;
        fs::write(&self.cache_path, json)?;
        
        println!("Successfully generated prompts for the next 7 days!");
        Ok(())
    }
    
    fn collect_recent_notes(&self, days: i64) -> Result<Vec<(String, String)>, Box<dyn Error>> {
        let mut notes = Vec::new();
        let today = Local::now().date_naive();
        
        for i in 0..days {
            let date = today - chrono::Duration::days(i);
            let filename = format!("{}.md", date.format("%Y-%m-%d"));
            let filepath = self.notes_dir.join(&filename);
            
            if filepath.exists() {
                if let Ok(content) = fs::read_to_string(&filepath) {
                    // Skip if file is mostly empty (just header)
                    let lines: Vec<&str> = content.lines().collect();
                    if lines.len() > 2 && lines[2..].join("").trim().len() > 50 {
                        notes.push((date.format("%Y-%m-%d").to_string(), content));
                    }
                }
            }
        }
        
        Ok(notes)
    }
    
    fn analyze_and_generate(&self, notes: Vec<(String, String)>) -> Result<HashMap<String, DailyPrompt>, Box<dyn Error>> {
        // Combine recent notes for analysis
        let notes_summary = notes.iter()
            .map(|(date, content)| {
                let preview = content.lines()
                    .skip(2) // Skip header
                    .take(5) // First 5 lines
                    .collect::<Vec<_>>()
                    .join(" ");
                format!("{}: {}", date, preview)
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        // Create prompt for Claude
        let system_prompt = "You are helping generate personalized daily journal prompts based on someone's recent journal entries. Analyze the themes, emotions, and patterns in their writing to create thoughtful, relevant prompts that encourage deeper reflection and personal growth.";
        
        let user_prompt = format!(
            "Based on these recent journal entries, generate 7 unique daily prompts for the next week. Each prompt should be:\n\
            - Personalized based on themes you notice\n\
            - Encouraging deeper reflection\n\
            - Different from each other\n\
            - About 10-20 words\n\n\
            Recent entries:\n{}\n\n\
            Return a JSON array with exactly 7 objects, each having:\n\
            - \"date\": \"YYYY-MM-DD\" (starting from tomorrow)\n\
            - \"prompt\": \"The prompt text\"\n\
            - \"theme\": \"Brief theme (1-3 words)\"\n\
            - \"context\": \"Optional brief explanation\"",
            notes_summary
        );
        
        // Call Anthropic API
        let client = Client::new();
        let request = AnthropicRequest {
            model: "claude-3-haiku-20240307".to_string(),
            max_tokens: 1000,
            messages: vec![
                Message {
                    role: "user".to_string(),
                    content: format!("{}\n\n{}", system_prompt, user_prompt),
                },
            ],
        };
        
        let response = client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()?;
        
        if !response.status().is_success() {
            let error_text = response.text().unwrap_or_else(|_| "Unknown error".to_string());
            eprintln!("API Error Response: {}", error_text);
            return Err(format!("API request failed: {}", error_text).into());
        }
        
        let api_response: AnthropicResponse = response.json()?;
        let json_str = api_response.content.get(0)
            .ok_or("No response content")?
            .text.clone();
        
        // Parse the JSON response
        let prompt_array: Vec<serde_json::Value> = serde_json::from_str(&json_str)?;
        let mut prompts = HashMap::new();
        
        for (i, prompt_obj) in prompt_array.iter().enumerate() {
            let date = Local::now().date_naive() + chrono::Duration::days((i + 1) as i64);
            let date_str = date.format("%Y-%m-%d").to_string();
            
            let prompt = DailyPrompt {
                prompt: prompt_obj["prompt"].as_str().unwrap_or("What are you grateful for today?").to_string(),
                theme: prompt_obj["theme"].as_str().unwrap_or("reflection").to_string(),
                context: prompt_obj["context"].as_str().map(|s| s.to_string()),
            };
            
            prompts.insert(date_str, prompt);
        }
        
        Ok(prompts)
    }
}

// Public function to get prompt for a specific date
pub fn get_ai_prompt(config: &Config, date: &NaiveDate) -> Option<String> {
    if let Ok(generator) = PromptGenerator::new(config) {
        if let Some(daily_prompt) = generator.load_cached_prompt(date) {
            return Some(daily_prompt.prompt);
        }
    }
    None
}