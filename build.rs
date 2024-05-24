use vergen_gix::{BuildBuilder, GixBuilder};

fn main() {
    // Configure vergen
    vergen_gix::Emitter::default()
        .add_instructions(&BuildBuilder::all_build().unwrap())
        .unwrap()
        .add_instructions(&GixBuilder::all_git().unwrap())
        .unwrap()
        .emit()
        .unwrap();
}
