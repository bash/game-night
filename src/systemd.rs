use rocket::fairing::{Fairing, Info, Kind};
use rocket::{async_trait, warn, Orbit, Rocket};
use std::iter;
use systemd::daemon::notify;

pub(crate) struct SystemdNotify;

#[async_trait]
impl Fairing for SystemdNotify {
    fn info(&self) -> Info {
        Info {
            name: "sd_notify(...)",
            kind: Kind::Liftoff | Kind::Shutdown,
        }
    }

    async fn on_liftoff(&self, _rocket: &Rocket<Orbit>) {
        let unset_environment = false;

        if let Err(e) = notify(
            unset_environment,
            &mut [("READY", "1"), ("STATUS", "🚀 Rocket has launched")].iter(),
        ) {
            warn!("Failed to notify systemd: {}", e);
        }
    }

    async fn on_shutdown(&self, _rocket: &Rocket<Orbit>) {
        let unset_environment = false;
        if let Err(e) = notify(unset_environment, iter::once(&("STOPPING", "1"))) {
            warn!("Failed to notify systemd: {}", e);
        }
    }
}
