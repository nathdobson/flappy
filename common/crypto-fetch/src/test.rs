use crate::fetch_certificate_list_sha256;

#[tokio::test]
async fn test() {
    for x in fetch_certificate_list_sha256("u8c6afc1.ala.us-east-1.emqxsl.com".to_string(), 8883)
        .await
        .unwrap()
    {
        print!("{:02X}",x);
    }
    println!();
}
