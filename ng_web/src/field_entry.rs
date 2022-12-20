use web_sys::HtmlInputElement;
use yew::events::Event;
use yew::prelude::*;
use yew::{AttrValue, Properties};

#[derive(Properties, PartialEq)]
pub struct FieldEntryProps {
    pub label: AttrValue,
    pub value: AttrValue,
    pub placeholder: AttrValue,
    pub error: AttrValue,
    pub onchange: Callback<String>,
}

#[function_component(FieldEntry)]
pub fn field_entry(props: &FieldEntryProps) -> Html {
    let target_input_value = |e: &Event| {
        let input: HtmlInputElement = e.target_unchecked_into();
        input.value()
    };

    let onchange = {
        let onchange = props.onchange.clone();

        move |e: Event| {
            if e.type_() == "change" {
                let value = target_input_value(&e);
                onchange.emit(value)
            }
        }
    };

    let maybe_display_error = move || -> Html {
        if !props.error.is_empty() {
            html! {
                <p class="help is-danger" >
                { props.error.clone() }
                </p>
            }
        } else {
            html! {}
        }
    };

    html! {
        <div class="field">
            <label>{ props.label.clone() }</label>
            <div class="control">
                <input
                    type="text"
                    value={ props.value.clone() }
                    class="input"
                    placeholder={ props.placeholder.clone() }
                    { onchange }
                />
            </div>
            { maybe_display_error() }
        </div>
    }
}
