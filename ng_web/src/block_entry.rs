use crate::field_entry::FieldEntry;
use log::debug;
use std::rc::Rc;
use std::str::FromStr;
use yew::prelude::*;
use yew::{function_component, Reducible};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockEntryState {
    pub block: Option<String>,
    pub block_error: Option<String>,
}

pub enum BlockEntryAction {
    SetBlock(String),
    SetBlockError(String),
    Clear,
}

impl Reducible for BlockEntryState {
    type Action = BlockEntryAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            BlockEntryAction::SetBlock(block) => BlockEntryState {
                block: if !block.is_empty() { Some(block) } else { None },
                block_error: self.block_error.clone(),
            }
            .into(),
            BlockEntryAction::SetBlockError(block_error) => BlockEntryState {
                block: None,
                block_error: Some(block_error),
            }
            .into(),
            BlockEntryAction::Clear => BlockEntryState {
                block: None,
                block_error: None,
            }
            .into(),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct BlockEntryProps {
    pub onsetblock: Callback<u32>,
}

#[function_component(BlockEntry)]
pub fn block_entry(props: &BlockEntryProps) -> Html {
    let onsetblock = props.onsetblock.clone();

    let state = use_reducer(|| BlockEntryState {
        block: None,
        block_error: None,
    });

    let onchange_block = {
        let state = state.clone();
        Callback::from(move |value: String| {
            debug!("new block value: {}", &value);
            state.dispatch(BlockEntryAction::SetBlock(value));
        })
    };

    let onclick_block = {
        let state = state.clone();
        move |_: MouseEvent| {
            if state.block.is_some() {
                let block = state.block.clone().unwrap();
                if let Ok(decimal_block) = u32::from_str(block.as_str()) {
                    onsetblock.emit(decimal_block);
                    state.dispatch(BlockEntryAction::Clear);
                } else {
                    state.dispatch(BlockEntryAction::SetBlockError(
                        format!("Block '{}' is not a valid decimal.", &block).to_string(),
                    ))
                }
            } else {
                if state.block.is_none() {
                    state.dispatch(BlockEntryAction::SetBlockError(
                        "Block must be set.".to_string(),
                    ));
                }
            }
        }
    };

    let block_value = state.block.clone().unwrap_or_default();
    let block_error = state.block_error.clone().unwrap_or_default();

    html! {
        <div class="block">
            <FieldEntry label={"Target Block"} value={block_value} placeholder={"eg. 768159"} error={block_error} onchange={onchange_block.clone()} />
            <div class="control">
                <button class = "button is-link" onclick={ onclick_block }>{"Set"}</button>
            </div>
        </div>
    }
}
