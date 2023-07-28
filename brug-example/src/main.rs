use tokio::sync::oneshot;
use brug::{async_trait, Performer};
use brug::tokio::OneShot;
use std::any::Any;

pub struct Gamer;

#[brug::performer]
impl Gamer {
    async fn good(&mut self, nice: u32) -> bool {
        nice == 4
    }

    fn good_job_media(&mut self, _wow: Box<dyn Any + Send>) -> String {
        todo!()
    }
}

#[tokio::main]
async fn main() {
    let (s, r) = oneshot::channel::<bool>();
    tokio::spawn(async move {
        let x = GamerCommand::<OneShot>::Good(3, s);
        let mut g = Gamer;
        g.perform(x).await;
    });


    let v = r.await.expect(":(");
    assert!(!v);

    pub struct Wrapper(Gamer);

    #[async_trait]
    impl GamerFacadeMut<OneShot> for Wrapper {
        async fn handle(&mut self, command: GamerCommand<OneShot>) {
            self.0.perform(command).await;
        }
    }

    let mut w = Wrapper(Gamer);

    assert!(!w.good(3).await);
}
