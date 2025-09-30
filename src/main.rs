use firewheel::{FirewheelContext, diff::Memo, error::UpdateError, node::NodeID};
use firewheel::{
    StreamInfo,
    atomic_float::AtomicF32,
    channel_config::{ChannelConfig, ChannelCount},
    collector::ArcGc,
    diff::{Diff, ParamPath, Patch},
    event::{NodeEventType, ParamData, ProcEvents},
    node::{
        AudioNode, AudioNodeInfo, AudioNodeProcessor, ConstructProcessorContext, ProcBuffers,
        ProcExtra, ProcInfo, ProcessStatus,
    },
};
use std::sync::Arc;

struct DummyInner;

#[derive(Debug, Default)]
struct SharedState;

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct DummyConfig;

#[derive(Diff, Patch, Clone)]
struct DummyNode {
    pub dummy: Option<ArcGc<DummyInner>>,
}

#[derive(Clone)]
pub struct DummyState {
    shared_state: ArcGc<SharedState>,
}

impl DummyState {
    fn new() -> Self {
        Self {
            shared_state: ArcGc::new(SharedState::default()),
        }
    }
}

impl Default for DummyNode {
    fn default() -> Self {
        Self { dummy: None }
    }
}

impl AudioNode for DummyNode {
    type Configuration = DummyConfig;

    fn info(&self, _config: &Self::Configuration) -> AudioNodeInfo {
        AudioNodeInfo::new()
            .debug_name("dummy")
            .channel_config(ChannelConfig {
                num_inputs: ChannelCount::ZERO,
                num_outputs: ChannelCount::STEREO,
            })
            .custom_state(DummyState::new())
    }

    fn construct_processor(
        &self,
        config: &Self::Configuration,
        cx: ConstructProcessorContext,
    ) -> impl AudioNodeProcessor {
        let custom_state = cx.custom_state::<DummyState>().unwrap();
        Processor {
            params: self.clone(),
            shared_state: ArcGc::clone(&custom_state.shared_state),
            config: *config,
        }
    }
}

impl DummyNode {
    fn set_dummy(&mut self) {
        self.dummy = Some(ArcGc::new_unsized(|| Arc::new(DummyInner)))
    }

    fn sync_dummy(&mut self) -> NodeEventType {
        NodeEventType::Param {
            data: ParamData::any(self.dummy.clone()),
            path: ParamPath::Single(0),
        }
    }
}

struct Processor {
    params: DummyNode,
    shared_state: ArcGc<SharedState>,
    config: DummyConfig,
}

impl AudioNodeProcessor for Processor {
    // The realtime process method.
    fn process(
        &mut self,
        // Information about the process block.
        info: &ProcInfo,
        // The buffers of data to process.
        buffers: ProcBuffers,
        // The list of events for our node to process.
        events: &mut ProcEvents,
        // Extra buffers and utilities.
        _extra: &mut ProcExtra,
    ) -> ProcessStatus {
        for patch in events.drain_patches::<DummyNode>() {
            match patch {
                DummyNodePatch::Dummy(d) => {
                    assert!(d.is_some());
                }
            }
            // self.params.apply(patch);
        }
        ProcessStatus::Bypass
    }
}

fn main() {
    let mut cx = FirewheelContext::new(Default::default());
    cx.start_stream(Default::default()).unwrap();

    let mut dummy_node = DummyNode::default();
    let dummy_node_id = cx.add_node(dummy_node.clone(), None);
    let graph_out_node_id = cx.graph_out_node_id();
    cx.connect(dummy_node_id, graph_out_node_id, &[(0, 0)], false)
        .unwrap();
    dummy_node.set_dummy();
    cx.queue_event_for(dummy_node_id, dummy_node.sync_dummy());

    loop {
        if let Err(e) = cx.update() {
            log::error!("{:?}", &e);

            if let UpdateError::StreamStoppedUnexpectedly(_) = e {
                panic!("Stream stopped unexpectedly!");
            }
        }
    }
}
