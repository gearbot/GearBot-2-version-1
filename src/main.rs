use dotenv;

fn main() {
    println!("Hello there, ferris!");

    let token = dotenv::var("token").unwrap();
}
