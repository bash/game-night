use lettre::message::MultiPart;
use lettre::{FileTransport, Message, Transport};
use tera::{Context, Tera};

fn main() {
    let template_name = std::env::args()
        .nth(1)
        .expect("Exactly one argument is required");
    let mut tera = Tera::new("emails/*.tera").unwrap();
    tera.build_inheritance_chains().unwrap();
    let context = build_context();
    let email = Message::builder()
        .from("Tau's Game Night <game-night@tau.soy>".parse().unwrap())
        .to("Example Recipient <recipient@example.com>".parse().unwrap())
        .subject("Tau's Game Night")
        .multipart(MultiPart::alternative_plain_html(
            "TODO: plain".to_string(),
            tera.render(&template_name, &context).unwrap(),
        ))
        .unwrap();
    FileTransport::new("emails").send(&email).unwrap();
}

fn build_context() -> Context {
    let mut context = Context::new();
    context.insert("greeting", "Hi");
    context.insert("name", "Tau");
    context.insert("poll_url", "https://www.example.com");
    context.insert("poll_close_date", "30. September 2023");
    context.insert("poll_close_time", "5:30 PM");
    context
}
