use super::*;
use crate::SessionId;
use crate::execution::{Action, Reply};

impl Node {
    /// Tools may name a machine, while terminal rows carry its stable home ID.
    /// The empty target and our own ID remain local even without a device row.
    pub(super) fn execution_target(
        &self,
        machine: &str,
    ) -> Result<(String, khor_core::DeviceId), String> {
        if machine.is_empty() {
            return Ok((self.name().to_owned(), self.device()));
        }
        if let Ok(home) = crate::device_id_from_hex(machine) {
            if home == self.device() {
                return Ok((self.name().to_owned(), home));
            }
            return self.recovery_peer(home).map(|device| (device.name, home));
        }
        self.resolve(machine)
    }

    pub async fn execution_on(&self, machine: &str, action: Action) -> Result<Reply, String> {
        action.validate()?;
        if let Some(reply) = self
            .via_serve(ipc::Op::ExecutionOn {
                machine: machine.into(),
                action: action.clone(),
            })
            .await
        {
            return match reply? {
                ipc::Reply::Execution { value } => Ok(value),
                ipc::Reply::Refused { why } => Err(why),
                other => Err(msg::serve_non_answer(format_args!("{other:?}"))),
            };
        }
        let (_, home) = self.execution_target(machine)?;
        if home == self.device() {
            return self.execution_here(action).await;
        }
        let ep = self
            .bind_client(self.secret_key().clone())
            .await
            .map_err(|e| e.to_string())?;
        let result = self.execution_with(&ep, machine, action).await;
        ep.close().await;
        result
    }

    pub(super) async fn execution_with(
        &self,
        ep: &iroh::Endpoint,
        machine: &str,
        action: Action,
    ) -> Result<Reply, String> {
        action.validate()?;
        let (channel, home) = self.execution_target(machine)?;
        if home == self.device() {
            return self.execution_here(action).await;
        }
        if self.sat_and_sponsor(&channel)?.is_some() {
            return Err("Command tasks and terminal control require a Khor host".into());
        }
        let device = self.recovery_peer(home)?;
        match self
            .ask_on(ep, &device, &Request::Execution { action })
            .await?
        {
            Response::Execution { value } => Ok(value),
            Response::Refused { why } => Err(why),
            other => Err(msg::peer_non_answer(format_args!("{other:?}"))),
        }
    }

    pub(super) async fn execution_here(&self, action: Action) -> Result<Reply, String> {
        self.refuse_unless_hosting()?;
        let root = self.root().to_path_buf();
        tokio::task::spawn_blocking(move || crate::execution::handle(&root, action))
            .await
            .map_err(|e| e.to_string())?
    }

    pub async fn command_task_on(
        &self,
        machine: &str,
        request: crate::command_task::Request,
    ) -> Result<crate::command_task::Reply, String> {
        match self
            .execution_on(machine, Action::CommandTask(request))
            .await?
        {
            Reply::CommandTask(value) => Ok(value),
            _ => Err("Unexpected command task response".into()),
        }
    }

    pub async fn terminal_on(
        &self,
        machine: &str,
        request: crate::terminal_control::Request,
    ) -> Result<crate::terminal_control::Reply, String> {
        let target = if machine.is_empty() {
            self.every_session()?
                .into_iter()
                .find(|v| v.session.id.0 == request.session())
                .map(|v| v.session.home.hex())
                .ok_or_else(|| format!("No such session: {}", request.session()))?
        } else {
            machine.to_owned()
        };
        match self
            .execution_on(&target, Action::Terminal(request))
            .await?
        {
            Reply::Terminal(value) => Ok(value),
            _ => Err("Unexpected terminal response".into()),
        }
    }

    pub async fn ended_anywhere(
        &self,
        id: &SessionId,
    ) -> Result<Option<crate::live::Ended>, String> {
        match self
            .terminal_on(
                "",
                crate::terminal_control::Request::Ended {
                    session: id.0.clone(),
                },
            )
            .await?
        {
            crate::terminal_control::Reply::Ended { value } => Ok(value),
            _ => Err("Unexpected ended terminal response".into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_targets_accept_empty_and_self_id_but_refuse_an_unknown_home() {
        let root = crate::testdir::TempTree::fresh("execution-target");
        let node = Node::open_as(root.path(), "execution-target").unwrap();
        for target in ["", node.device_str(), node.name()] {
            assert_eq!(
                node.execution_target(target).unwrap(),
                (node.name().to_owned(), node.device())
            );
        }
        let unknown = khor_core::DeviceId([0; 32]);
        assert_ne!(unknown, node.device());
        assert!(node.execution_target(&unknown.hex()).is_err());
    }

    #[test]
    fn execution_home_ids_do_not_follow_another_devices_colliding_label() {
        let root = crate::testdir::TempTree::fresh("execution-label-collision");
        let node = Node::open_as(root.path(), "execution-caller").unwrap();
        let peer = khor_core::DeviceId([4; 32]);
        let loaded = node.devices_loaded().unwrap();
        loaded
            .doc
            .upsert(&peer.hex(), "execution-peer", &[])
            .unwrap();
        loaded
            .doc
            .set_label(node.device_str(), "execution-peer")
            .unwrap();
        let mut store = loaded.store;
        store.flush(&loaded.doc).unwrap();
        assert_eq!(
            node.execution_target("execution-peer").unwrap().1,
            node.device(),
            "a human-visible label retains its usual precedence"
        );
        assert_eq!(
            node.execution_target(&peer.hex()).unwrap(),
            ("execution-peer".to_owned(), peer),
            "the stable home still names the actual remote device"
        );
    }
}
