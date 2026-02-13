use quack_check::{chunk_plan::ChunkPlan, config::Config};

#[test]
fn chunk_plan_basic() {
    let cfg = Config::default();
    let plan = ChunkPlan::from_page_count(&cfg, 101);
    assert!(!plan.chunks.is_empty());
    assert_eq!(plan.chunks[0].start_page, 1);
    assert_eq!(plan.chunks.last().unwrap().end_page, 101);
}
