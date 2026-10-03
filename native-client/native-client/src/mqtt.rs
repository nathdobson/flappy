use crate::Error;
use crate::FlapTestCommand;
use std::iter;
use tokio::sync::mpsc;

pub async fn run_flap_test(flap_test: &FlapTestCommand) -> Result<(), Error> {
    use mqtt_client_tokio::MqttBuilder;
    use mqtt_client_tokio::TokioMqttPublishRequest;
    use mqtt_core::protocol::Qos;
    use protocol::display::DisplayRequest;
    use tokio::io::AsyncBufReadExt;
    use tokio::io::BufReader;
    let (req_tx, req_rx) = mpsc::channel(1);
    let topic = flap_test.topic.clone();
    let module_index = flap_test.index.clone();
    let step = flap_test.step.unwrap_or(1);
    tokio::spawn(async move {
        let mut reader = BufReader::new(tokio::io::stdin()).lines();
        let glyphs = [
            " ", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
            "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "$", "&", "#", "0", "1", "2", "3",
            "4", "5", "6", "7", "8", "9", ":", ".", "-", "?", "!",
        ];
        let mut index = 0;
        while let Some(line) = reader.next_line().await.unwrap() {
            let mut message = iter::repeat_n(" ".to_string(), 10).collect::<Vec<String>>();
            for module_index in &module_index {
                message[*module_index] = format!("{}", glyphs[index % glyphs.len()]);
            }
            let message = message.into_iter().collect::<String>();
            println!("{:?}", message);
            index += step;
            req_tx
                .send(TokioMqttPublishRequest {
                    qos: Qos::AtMostOnce,
                    retain: false,
                    topic: format!("{}/request", topic),
                    payload: serde_json_core::to_vec::<DisplayRequest, 1024>(&DisplayRequest::Run(
                        heapless::String::try_from(&*message).unwrap(),
                    ))
                    .unwrap()
                    .to_vec(),
                })
                .await
                .unwrap();
        }
    });

    let mqtt = MqttBuilder {
        host: flap_test.host.clone(),
        port: flap_test.port,
        username: flap_test.username.clone(),
        password: flap_test.password.clone(),
        keepalive: 60,
        client_id: flap_test.client_id.clone(),
    }
    .run(req_rx)
    .await
    .unwrap();
    Ok(())
}
