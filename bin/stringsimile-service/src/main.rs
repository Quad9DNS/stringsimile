use tokio::io::{self, AsyncBufReadExt, BufReader};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let reader_task = tokio::spawn(async move {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line().await.expect("reading failed") {
            println!("length = {}", line.len())
        }
    });
    reader_task.await.map_err(Box::new)?;
    Ok(())
}
