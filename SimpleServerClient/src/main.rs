use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio::time::sleep;

#[derive(Serialize)]
struct MathRequest {
    a: f64,
    b: f64,
}

#[derive(Deserialize)]
struct MathResponse {
    result: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(Client::new());
    let operations = [
        "http://localhost:8080/api/add",
        "http://localhost:8080/api/subtract", 
        "http://localhost:8080/api/multiply",
        "http://localhost:8080/api/divide"
    ];
    let mut interval = Duration::from_millis(100);
    
    let counter = Arc::new(Mutex::new(0));
    let success_count = Arc::new(Mutex::new(0));
    let mut last_report = Instant::now();
    let mut op_index = 0;
    
    loop {
        let start = Instant::now();
        
        let mut tasks = vec![];
        for _ in 0..10 {
            let client = Arc::clone(&client);
            let counter = Arc::clone(&counter);
            let success_count = Arc::clone(&success_count);
            let url = operations[op_index % operations.len()].to_string();
            op_index += 1;
            
            let task = tokio::spawn(async move {
                let request = MathRequest { a: 10.0, b: 2.0 };
                match client.post(&url).json(&request).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            if response.json::<MathResponse>().await.is_ok() {
                                *counter.lock().await += 1;
                                *success_count.lock().await += 1;
                            }
                        }
                    }
                    Err(_) => {}
                }
            });
            tasks.push(task);
        }
        
        for task in tasks {
            let _ = task.await;
        }
        
        if last_report.elapsed() >= Duration::from_secs(1) {
            let count = *counter.lock().await;
            let success = *success_count.lock().await;
            println!("Requests per second: {}", count);
            
            if success > 0 {
                interval = Duration::from_nanos((interval.as_nanos() as f64 * 0.833) as u64);
            }
            
            *counter.lock().await = 0;
            *success_count.lock().await = 0;
            last_report = Instant::now();
        }
        
        let elapsed = start.elapsed();
        if elapsed < interval {
            sleep(interval - elapsed).await;
        }
    }
}