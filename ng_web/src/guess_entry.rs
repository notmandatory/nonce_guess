use crate::field_entry::FieldEntry;
use log::debug;
use ng_model::Guess;
use std::rc::Rc;
use yew::prelude::*;
use yew::{function_component, Reducible};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuessEntryState {
    pub name: Option<String>,
    pub nonce: Option<String>,
    pub name_error: Option<String>,
    pub nonce_error: Option<String>,
}

pub enum GuessEntryAction {
    SetName(String),
    SetNonce(String),
    SetNameError(String),
    SetNonceError(String),
    Clear,
}

impl Reducible for GuessEntryState {
    type Action = GuessEntryAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            GuessEntryAction::SetName(name) => GuessEntryState {
                name: if !name.is_empty() { Some(name) } else { None },
                nonce: self.nonce.clone(),
                name_error: self.name_error.clone(),
                nonce_error: self.nonce_error.clone(),
            }
            .into(),
            GuessEntryAction::SetNonce(nonce) => GuessEntryState {
                name: self.name.clone(),
                nonce: if !nonce.is_empty() { Some(nonce) } else { None },
                name_error: self.name_error.clone(),
                nonce_error: self.nonce_error.clone(),
            }
            .into(),
            GuessEntryAction::SetNameError(name_error) => GuessEntryState {
                name: None,
                nonce: self.nonce.clone(),
                name_error: Some(name_error),
                nonce_error: self.nonce_error.clone(),
            }
            .into(),
            GuessEntryAction::SetNonceError(nonce_error) => GuessEntryState {
                name: self.name.clone(),
                nonce: None,
                name_error: self.name_error.clone(),
                nonce_error: Some(nonce_error),
            }
            .into(),
            GuessEntryAction::Clear => GuessEntryState {
                name: None,
                nonce: None,
                name_error: None,
                nonce_error: None,
            }
            .into(),
        }
    }
}

#[derive(Properties, PartialEq)]
pub struct GuessEntryProps {
    pub block: Option<u32>,
    pub onaddguess: Callback<Guess>,
}

#[function_component(GuessEntry)]
pub fn guess_entry(props: &GuessEntryProps) -> Html {
    let onaddguess = props.onaddguess.clone();

    let state = use_reducer(|| GuessEntryState {
        name: None,
        nonce: None,
        name_error: None,
        nonce_error: None,
    });

    let onchange_name = {
        let state = state.clone();
        Callback::from(move |value: String| {
            debug!("new name value: {}", &value);
            state.dispatch(GuessEntryAction::SetName(value));
        })
    };

    let onchange_nonce = {
        let state = state.clone();
        Callback::from(move |value: String| {
            debug!("new nonce value: {}", &value);
            state.dispatch(GuessEntryAction::SetNonce(value));
        })
    };

    let block = props.block.clone();
    let onclick = {
        let state = state.clone();
        move |_: MouseEvent| {
            if state.name.is_some() && state.nonce.is_some() && block.is_some() {
                let nonce = state.nonce.clone().unwrap();
                if let Ok(decimal_nonce) = u32::from_str_radix(nonce.as_str(), 16) {
                    onaddguess.emit(Guess {
                        block: block.unwrap(),
                        name: state.name.clone().unwrap(),
                        nonce: decimal_nonce,
                    });
                    state.dispatch(GuessEntryAction::Clear);
                } else {
                    state.dispatch(GuessEntryAction::SetNonceError(
                        format!("Nonce '{}' is not valid hex.", &nonce).to_string(),
                    ))
                }
            } else {
                if state.name.is_none() {
                    state.dispatch(GuessEntryAction::SetNameError(
                        "Name must be set.".to_string(),
                    ));
                }
                if state.nonce.is_none() {
                    state.dispatch(GuessEntryAction::SetNonceError(
                        "Nonce must be set.".to_string(),
                    ));
                }
            }
        }
    };

    let name_value = state.name.clone().unwrap_or_default();
    let nonce_value = state.nonce.clone().unwrap_or_default();
    let name_error = state.name_error.clone().unwrap_or_default();
    let nonce_error = state.nonce_error.clone().unwrap_or_default();

    html! {
        <div class="block">
            <FieldEntry label={"Name"} value={name_value} placeholder={"eg. Steve"} error={name_error} onchange={onchange_name.clone()} />
            <FieldEntry label={"Nonce Guess, Hex"} value={nonce_value} placeholder={"8 characters, use only 0-9 and A-F, eg. 2FC683D5"} error={nonce_error} onchange={onchange_nonce.clone()} />
            <div class="control">
                <button class = "button is-link" onclick={ onclick }>{"Add"}</button>
            </div>
        </div>
    }
}
