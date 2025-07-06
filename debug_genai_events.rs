use genai::chat::ChatStreamEvent;

fn main() {
    // Print debug info about ChatStreamEvent variants
    println!("Checking ChatStreamEvent variants...");
    
    // This will show us what variants are available at compile time
    match ChatStreamEvent::Start {
        ChatStreamEvent::Start => println!("Start variant exists"),
        ChatStreamEvent::End { .. } => println!("End variant exists"),
        ChatStreamEvent::ContentStart { .. } => println!("ContentStart variant exists"),
        ChatStreamEvent::ContentDelta { .. } => println!("ContentDelta variant exists"),
        _ => println!("Other variants exist"),
    }
}