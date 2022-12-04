use web_sys::HtmlInputElement;
use thousands::Separable;
use yew::prelude::*;

pub enum Msg {
    AddGuess,
    SetNonce,
}

pub struct App {
    guesses: Vec<(String, u32)>,
    target_nonce: Option<u32>,
    guess_refs: (NodeRef, NodeRef),
    target_nonce_ref: NodeRef,
    name_error: String,
    guess_error: String,
    target_nonce_error: String,
}

fn error_is_hidden(error: &String) -> Option<String> {
    if error.is_empty() {
        Some("is-hidden".to_string())
    } else {
        None
    }
}

impl App {
    fn sort_guesses(&mut self) {
        if self.target_nonce.is_some() {
            let target = self.target_nonce.unwrap();
            self.guesses.sort_by(|a, b| {
                let target_a = target.abs_diff(a.1);
                let target_b = target.abs_diff(b.1);
                target_a.cmp(&target_b)
            })
        }
    }
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self {
            guesses: Vec::new(),
            target_nonce: None,
            guess_refs: (NodeRef::default(), NodeRef::default()),
            target_nonce_ref: NodeRef::default(),
            name_error: "".to_string(),
            guess_error: "".to_string(),
            target_nonce_error: "".to_string(),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::AddGuess => {
                let name_ref = &self.guess_refs.0.cast::<HtmlInputElement>().unwrap();
                let guess_ref = &self.guess_refs.1.cast::<HtmlInputElement>().unwrap();

                let name_value = name_ref.value();
                let guess_value = guess_ref.value();

                let mut name_error = Vec::default();
                if name_value.is_empty() {
                    name_error.push("Name can not be empty.".to_string());
                    self.name_error = name_error.join(", ");
                }

                let mut guess_error = Vec::default();
                if guess_value.is_empty() {
                    guess_error.push("Guess can not be empty.".to_string());
                    self.guess_error = guess_error.join(", ");
                }

                if name_error.is_empty() && guess_error.is_empty() {
                    let guess_value_u32 = u32::from_str_radix(guess_value.as_str(), 16).unwrap();
                    self.guesses.push((name_value, guess_value_u32));
                    self.name_error = String::default();
                    self.guess_error = String::default();
                    name_ref.set_value("");
                    guess_ref.set_value("");
                }
                self.sort_guesses();
                true
            },
            Msg::SetNonce => {
                let target_nonce_ref = &self.target_nonce_ref.cast::<HtmlInputElement>().unwrap();
                let target_nonce_value = target_nonce_ref.value();
                let mut target_nonce_error = Vec::default();
                if target_nonce_value.is_empty() {
                    target_nonce_error.push("Target nonce can not be empty.".to_string());
                    self.target_nonce_error = target_nonce_error.join(", ");
                }

                if target_nonce_error.is_empty() {
                    let target_nonce_value_u32 = u32::from_str_radix(target_nonce_value.as_str(), 16).unwrap();
                    self.target_nonce = Some(target_nonce_value_u32);
                    self.target_nonce_error = String::default();
                    target_nonce_ref.set_value("");
                }
                self.sort_guesses();
                true
            }
        }
    }

    fn view(&self, ctx: &Context<Self>) -> Html {

        html! {
            <div class="section">
                <div class="container">
                    <h1 class="title">{ "Guess the Next Block Nonce" }</h1>
                    <div class="columns">
                        <div class="column is-one-third">
                            <div class="block">
                                <div class="field">
                                    <label>{ "Name" }</label>
                                    <div class="control">
                                        <input
                                            type="text"
                                            ref={&self.guess_refs.0}
                                            class="input"
                                            placeholder="eg. Steve"
                                        />
                                    </div>
                                    <p class={classes!("help", "is-danger", error_is_hidden(&self.name_error))}>
                                        { self.name_error.clone() }
                                    </p>
                                </div>
                                <div class="field">
                                    <label>{ "Nonce Guess, Hex" }</label>
                                    <div class="control">
                                        <input
                                            type="text"
                                            ref={&self.guess_refs.1}
                                            class="input"
                                            placeholder="8 characters, use only 0-9 and A-F, eg. 2FC683D5"
                                        />
                                    </div>
                                    <p class={classes!("help", "is-danger", error_is_hidden(&self.guess_error))}>
                                        { self.guess_error.clone() }
                                    </p>
                                </div>
                                <div class="control">
                                    <button class = "button is-link" onclick={ctx.link().callback(|_| Msg::AddGuess)}>{"Add"}</button>
                                </div>
                            </div>
                            <div class="block">
                                <div class="field">
                                    <label>{ "Target Nonce, Hex" }</label>
                                    <div class="control">
                                        <input
                                            type="text"
                                            ref={&self.target_nonce_ref}
                                            class="input"
                                            placeholder="8 characters, use only 0-9 and A-F, eg. 2FC683D5"
                                        />
                                    </div>
                                    <p class={classes!("help", "is-danger", error_is_hidden(&self.target_nonce_error))}>
                                        { self.target_nonce_error.clone() }
                                    </p>
                                </div>
                                <div class="control">
                                    <button class = "button is-link" onclick={ctx.link().callback(|_| Msg::SetNonce)}>{"Set"}</button>
                                </div>
                            </div>
                        </div>
                        <div class="column">
                            {
                                if self.target_nonce.is_some() {
                                    html!{
                                        <div class="block">
                                            <div class="title is-4">{ "Target Nonce" }</div>
                                            <table class="table">
                                                <thead>
                                                    <tr>
                                                        <th>{ "Hex" }</th>
                                                        <th>{ "Dec" }</th>
                                                    </tr>
                                                </thead>
                                                <tbody>
                                                    <tr>
                                                        <td class="is-family-monospace">{ format!("{:08X}", &self.target_nonce.unwrap_or(0)) }</td>
                                                        <td class="is-family-monospace is-pulled-right">{ format!("{}", &self.target_nonce.unwrap_or(0).separate_with_commas()) }</td>
                                                    </tr>
                                                </tbody>
                                            </table>
                                        </div>
                                    }
                                } else {
                                    html!{ }
                                }
                            }
                            <div class="block">
                                <div class="title is-4">{ "Guesses" }</div>
                                <table class="table">
                                    <thead>
                                        <tr>
                                          <th>{ "Position" }</th>
                                          <th>{ "Name" }</th>
                                          <th>{ "Hex" }</th>
                                          <th>{ "Dec" }</th>
                                        </tr>
                                    </thead>
                                    <tbody>
                                    {
                                        self.guesses.clone().into_iter().enumerate().map(|(pos, (name, guess))| {
                                            html!{
                                            <tr key={ name.clone() }>
                                                <th>{ &pos }</th>
                                                <td>{ &name }</td>
                                                <td class="is-family-monospace">{ format!("{:08X}", &guess) }</td>
                                                <td class="is-family-monospace is-pulled-right">{ format!("{}", &guess.separate_with_commas()) }</td>
                                            </tr>
                                        }}).collect::<Html>()
                                    }
                                    </tbody>
                                </table>
                            </div>
                        </div>
                    </div>
                </div>
            </div>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
