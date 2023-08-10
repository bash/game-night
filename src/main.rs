use rocket::fs::FileServer;
use rocket::launch;

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", FileServer::from("public"))
}
