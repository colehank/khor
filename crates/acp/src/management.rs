//! A login connection has no session: authentication must work before session/new.
use super::*;
pub struct Management {
    pub events: mpsc::UnboundedReceiver<Event>,
    pub result: tokio::task::JoinHandle<Result<serde_json::Value, String>>,
}
impl Drop for Management {
    fn drop(&mut self) {
        self.result.abort();
    }
}
pub fn manage(command: &str, method: Option<String>) -> Result<Management, String> {
    let agent = AcpAgent::from_str(command).map_err(|e| e.to_string())?;
    Ok(manage_on(agent, method))
}

/// Run setup through caller-owned streams. The caller retains its process
/// lifecycle and confirms native exit after dropping this driver.
pub fn manage_on(
    agent: impl agent_client_protocol::ConnectTo<Client>,
    method: Option<String>,
) -> Management {
    let (tx, events) = mpsc::unbounded_channel();
    let result = tokio::spawn(async move {
        let elicits = tx.clone();
        let completed = tx.clone();
        let (result_tx, result_rx) = oneshot::channel();
        Client.builder().on_receive_request(
            async move|request:CreateElicitationRequest,responder,connection|{
                let(answer,rx)=oneshot::channel();let _=elicits.send(Event::Elicit(Elicitation{request,answer}));
                connection.spawn(async move{responder.respond(rx.await.unwrap_or_else(|_|CreateElicitationResponse::new(ElicitationAction::Cancel)))})
            },agent_client_protocol::on_receive_request!(),
        ).on_receive_notification(async move|n:agent_client_protocol::schema::v1::CompleteElicitationNotification,_cx|{let _=completed.send(Event::ElicitDone(n.elicitation_id.to_string()));Ok(())},agent_client_protocol::on_receive_notification!()).connect_with(agent,move|connection:ConnectionTo<Agent>|async move{
            let out=async{
                let mut init=InitializeRequest::new(ProtocolVersion::V1);init.client_capabilities=client_capabilities();
                let hello=connection.send_request(init).block_task().await.map_err(|e|e.to_string())?;
                if hello.protocol_version!=ProtocolVersion::V1{return Err("Unsupported ACP protocol version".into());}
                let facts=serde_json::json!({"capabilities":hello.agent_capabilities,"authMethods":hello.auth_methods,"agentInfo":hello.agent_info});
                if let Some(method)=method{
                    if !facts["authMethods"].as_array().is_some_and(|xs|xs.iter().any(|m|m["id"]==method&&m["type"].as_str().is_none_or(|t|t=="agent"))){return Err("This authentication method requires its advertised external login flow".into());}
                    connection.send_request(AuthenticateRequest::new(method)).block_task().await.map_err(|e|e.to_string())?;
                }
                Ok(facts)
            }.await;
            let _=result_tx.send(out);Ok(())
        }).await.map_err(|e|e.to_string())?;
        result_rx
            .await
            .map_err(|_| "The agent ended before answering".to_owned())?
    });
    Management { events, result }
}
