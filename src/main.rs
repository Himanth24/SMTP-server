use std::{ fmt::format, net::{TcpListener, TcpStream}, thread};
use std::io::{BufRead, BufReader, Write};
use std::fs::{create_dir_all, write};
use std::time::{SystemTime,UNIX_EPOCH};

fn handle_client(mut stream: TcpStream) {
    let peer_address = stream.peer_addr().unwrap().to_string();
    println!("New connection from {}", peer_address);

    let _ = stream.write_all(b"220 SampleSMTP Ready\r\n");

    let reader = BufReader::new(stream.try_clone().unwrap());   
    println!("{:?}", reader);

    let mut collecting_data = false;
    let mut data_lines = Vec::new();

    let mut mail_from = String::new();
    let mut rcpt_to = String::new();

    for line in reader.lines() {
        match line {
            Ok(line) => {
                print!("C: {}", line);
                if collecting_data {
                    if line == "." {
                        collecting_data = false;
                        let full_message= data_lines.join("\r\n");
                        if let Err(e) = save_mail(&rcpt_to, &full_message) {
                            eprintln!("Failed to save mail data: {}",e);
                            break;
                        }
                        if let Err(e) = stream.write_all(b"\r\n250 OK: message accepted\r\n") {
                            eprintln!("Failed to write response: {}", e);
                            break;
                        }
                        data_lines.clear();
                    } else {
                        data_lines.push(line.to_string());
                    }
                    continue;
                }

                if line.to_uppercase() == "DATA" {
                    collecting_data = true;
                    if let Err(e) = stream.write_all(b"354 End data with <CR><LF>.<CR><LF>\r\n") {
                        eprintln!("Error writing response: {}",e);
                        break;
                    }
                    continue;
                }
                let response = handle_smtp_command(&line, &mut mail_from, &mut rcpt_to);
                if let Err(e) = stream.write_all(response.as_bytes()) {
                    eprintln!("Failed to send response: {}",e);
                    break;
                }
                if line.to_uppercase() == "QUIT" {
                    break;
                }
            }
            Err(e) => {
                eprintln!("Read error: {}",e);
                break;
            }
        }
    }
    println!("Connection from {} closed.", peer_address);
}

fn handle_smtp_command(command: &str,mail_from: &mut String,rcpt_to: &mut String) -> String{ 
    let cmd = command.to_uppercase();
    let mut response = String::from("");
    if cmd.starts_with("HELO") {
        response = "250 Hello\r\n".to_string();
    } else if cmd.starts_with("MAIL FROM:") {
        *mail_from = command[10..].trim().to_string();
        response = "250 OK\r\n".to_string();
    } else if cmd.starts_with("RCPT TO:") {
        *rcpt_to = command[8..].trim().to_string();
        response = "250 OK\r\n".to_string();
    } else if cmd == "QUIT" {
        response = "221 Bye\r\n".to_string();
    } else {
        response = "502 Command not implemented\r\n".to_string();
    }
    response
}

fn save_mail(recipient: &str, message: &str) -> std::io::Result<()> {
    let recipient_sanitized = recipient.replace("@","_at_").replace("<", "").replace(">", "").replace(".","_");
    let dir = format!("mail/{}",recipient_sanitized);
    create_dir_all(&dir).unwrap();

    let system_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let filename = format!("{}/{}.eml",dir,system_time);
    let _ = write(filename, message);

    Ok(())
}

fn main() -> std::io::Result<()>{
    let listener = TcpListener::bind("127.0.0.1:2525").unwrap();
    println!("SMTP server listening on port 2525...");

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(|| {
                    handle_client(stream);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
    Ok(())
}
