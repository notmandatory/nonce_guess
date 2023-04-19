use crate::block_entry::BlockEntry;
use crate::guess_entry::GuessEntry;
use gloo_net::http::Request;
use log::{debug, info};
use ng_model::{check_for_duplicate_guess, sort_guesses_by_target_diff, Guess, Target};
use std::rc::Rc;
use std::str::FromStr;
use thousands::Separable;
use yew::prelude::*;
use yew::{function_component, Reducible};
use yew_router::prelude::*;

use base64::engine::general_purpose;
use base64::Engine;
use qrcode_generator::QrCodeEcc;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::window;

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/secure")]
    Secure,
    #[not_found]
    #[at("/404")]
    NotFound,
}

#[function_component(Secure)]
fn secure() -> Html {

    let state = use_reducer(|| AppState {
        target: None,
        guesses: None,
    });

    //let onclick = Callback::from(move |_| navigator.push(&Route::Home));
    let on_set_block = {
        let state = state.clone();
        Callback::from(move |block: u32| {
            debug!("new target block: {}", &block);
            {
                let state = state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let post_block_result = Request::post("/api/target")
                        .body(block)
                        .send()
                        .await
                        .unwrap();
                    debug!("post_block_result: {:?}", post_block_result);
                    if post_block_result.ok() {
                        state.dispatch(AppAction::SetBlock(block));
                    } else {
                        // TODO else display an error
                        debug!("set block error: {:?}", post_block_result);
                    }
                });
            }
        })
    };
    
    html! {
        <div class="section">
            <div class="container">
                <h1 class="title"><span>
                    <Link<Route> to={Route::Home}>
                        <img src="/img/apple-touch-icon.png" width="75" height="75"/>
                    </Link<Route>>
                </span></h1>
            </div>
            <div class="column">
                <BlockEntry onsetblock={ on_set_block } />
            </div>
        </div>
    }
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! {
            <Home />
        },
        Route::Secure => html! {
            <Secure />
        },
        Route::NotFound => html! { <h1>{ "404" }</h1> },
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppState {
    pub target: Option<Target>,
    pub guesses: Option<Vec<Guess>>,
}

pub enum AppAction {
    SetTarget(Target),
    SetGuesses(Vec<Guess>),
    SetBlock(u32),
    SetNonce(u32),
}

impl Reducible for AppState {
    type Action = AppAction;

    fn reduce(self: Rc<Self>, action: Self::Action) -> Rc<Self> {
        match action {
            AppAction::SetTarget(target) => AppState {
                guesses: self.guesses.clone(),
                target: Some(target),
            }
            .into(),
            AppAction::SetGuesses(guesses) => AppState {
                guesses: Some(guesses),
                target: self.target.clone(),
            }
            .into(),
            AppAction::SetBlock(block) => {
                AppState {
                    guesses: None,
                    target: Some(Target { block, nonce: None }),
                }
            }
            .into(),
            AppAction::SetNonce(nonce) => {
                AppState {
                    guesses: self.guesses.clone(),
                    target: Some(Target {
                        block: self.target.clone().unwrap().block,
                        nonce: Some(nonce),
                    }),
                }
            }
            .into(),
        }
    }
}

#[function_component(Main)]
pub fn app() -> Html {
    html! {
        <BrowserRouter>
            <Switch<Route> render={switch} /> // <- must be child of <BrowserRouter>
        </BrowserRouter>
    }
}

#[function_component(Home)]
fn home() -> Html {
    let state = use_reducer(|| AppState {
        target: None,
        guesses: None,
    });

    use_effect_with_deps(
        {
            let state = state.clone();
            move |_| {
                wasm_bindgen_futures::spawn_local(async move {
                    let fetched_target: Option<Target> = Request::get("/api/target")
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .ok();

                    if let Some(target) = &fetched_target {
                        info!("fetched target block: {}", target.block.to_string());
                        state.dispatch(AppAction::SetTarget(target.clone()));
                    }

                    let block_path = &fetched_target
                        .clone()
                        .map(|t| {
                            if t.nonce.is_some() {
                                format!("/{}", t.block)
                            } else {
                                String::default()
                            }
                        })
                        .unwrap_or_default();

                    let mut fetched_guesses: Vec<Guess> =
                        Request::get(format!("/api/guesses{}", block_path).as_str())
                            .send()
                            .await
                            .unwrap()
                            .json()
                            .await
                            .unwrap();

                    debug!("fetched guesses len: {}", fetched_guesses.len());
                    if let Some(Target {
                        block: _,
                        nonce: Some(nonce),
                    }) = fetched_target
                    {
                        sort_guesses_by_target_diff(fetched_guesses.as_mut_slice(), nonce);
                    }
                    state.dispatch(AppAction::SetGuesses(fetched_guesses.clone()));
                });
                || ()
            }
        },
        (),
    );

    let target = state.target.clone();

    let on_add_guess = {
        let state = state.clone();
        Callback::from(move |guess: Guess| {
            debug!("new player guess: {:?}", &guess);
            let state = state.clone();
            {
                wasm_bindgen_futures::spawn_local(async move {
                    let fetched_target: Option<Target> = Request::get("/api/target")
                        .send()
                        .await
                        .unwrap()
                        .json()
                        .await
                        .ok();

                    if let Some(target) = &fetched_target {
                        info!("fetched target block: {}", target.block.to_string());
                        state.dispatch(AppAction::SetTarget(target.clone()));
                    }

                    let block_path = &fetched_target
                        .clone()
                        .map(|t| {
                            if t.nonce.is_some() {
                                format!("/{}", t.block)
                            } else {
                                String::default()
                            }
                        })
                        .unwrap_or_default();
                    let mut fetched_guesses: Vec<Guess> =
                        Request::get(format!("/api/guesses{}", block_path).as_str())
                            .send()
                            .await
                            .unwrap()
                            .json()
                            .await
                            .unwrap();
                    let is_dupe: Result<_, _> =
                        check_for_duplicate_guess(fetched_guesses.as_mut_slice(), &guess);
                    match is_dupe {
                        Ok(s) => info!("Guess does not exist: {:?}", s),
                        Err(e) => {
                            let err_msg = format!("You cannot submit multiple guesses {:?}", e);
                            debug!("{}", err_msg);
                        }
                    };
                    let post_guess_result = Request::post("/api/guesses")
                        .json(&guess)
                        .unwrap()
                        .send()
                        .await
                        .unwrap();
                    debug!("post_guess_result: {:?}", post_guess_result);
                    if post_guess_result.ok() {
                        fetched_guesses =
                            Request::get(format!("/api/guesses{}", block_path).as_str())
                                .send()
                                .await
                                .unwrap()
                                .json()
                                .await
                                .unwrap();

                        debug!("fetched guesses len: {}", fetched_guesses.len());
                        if let Some(Target {
                            block: _,
                            nonce: Some(nonce),
                        }) = fetched_target
                        {
                            sort_guesses_by_target_diff(fetched_guesses.as_mut_slice(), nonce);
                        }
                        state.dispatch(AppAction::SetGuesses(fetched_guesses));
                    } else {
                        // TODO else display an error
                        debug!("add guess error: {:?}", post_guess_result);
                    }
                });
            }
        })
    };

    let document_url: String = window()
        .expect_throw("window is undefined")
        .document()
        .expect_throw("document is undefined")
        .url()
        .expect_throw("url is undefined");

    let url_qr_png: Vec<u8> =
        qrcode_generator::to_png_to_vec(document_url, QrCodeEcc::Low, 150).unwrap();
    let url_qr_base64 = general_purpose::STANDARD.encode(&url_qr_png);
    let url_qr_img_src = format!("data:image/png;base64,{}", url_qr_base64);

    let onclick_update = {
        let state = state.clone();
        Callback::from(move |_| {
            {
                let state = state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let get_nonce_result = Request::get("/api/target/nonce")
                        .send()
                        .await
                        .unwrap()
                        .text()
                        .await;

                    debug!("get_nonce_result: {:?}", get_nonce_result);
                    if let Ok(nonce) = get_nonce_result {
                        //let nonce = get_nonce_result.unwrap();
                        if !nonce.is_empty() {
                            let nonce = u32::from_str(nonce.as_str()).unwrap();
                            debug!("get_nonce_result: {}", nonce);
                            state.dispatch(AppAction::SetNonce(nonce));
                            if let Some(mut guesses) = state.guesses.clone() {
                                sort_guesses_by_target_diff(guesses.as_mut_slice(), nonce);
                                state.dispatch(AppAction::SetGuesses(guesses));
                            }
                        }
                    } else {
                        // TODO else display an error
                        debug!("get nonce error: {:?}", get_nonce_result);
                    }

                    if let Some(target) = state.target.clone() {
                        let get_guesses_result: Result<Vec<Guess>, _> =
                            Request::get(format!("/api/guesses").as_str())
                                .send()
                                .await
                                .unwrap()
                                .json()
                                .await;

                        if let Ok(mut guesses) = get_guesses_result {
                            if !guesses.is_empty() {
                                debug!("get_guesses_result: {:?}", guesses);
                                if let Some(nonce) = target.nonce {
                                    sort_guesses_by_target_diff(guesses.as_mut_slice(), nonce);
                                }
                                state.dispatch(AppAction::SetGuesses(guesses));
                            }
                        } else {
                            // TODO else display an error
                            debug!("get guesses error: {:?}", get_guesses_result);
                        }
                    }
                });
            }
        })
    };

    html! {
        <div class="section">
            <div class="container">
                <h1 class="title"><span>
                    <Link<Route> to={Route::Home}>
                        <img src="/img/apple-touch-icon.png" width="75" height="75"/>
                    </Link<Route>>
                    { "Guess the Block Nonce" }
                </span></h1>
             <img src={ url_qr_img_src } alt="URL QR Code" height="150" />
                <div class="columns">
                    <div class="column is-one-third">
                            <GuessEntry onaddguess={ on_add_guess }/>
                    </div>
                    <div class="column">
                        if let Some(target) = target {
                                <div class="block">
                                    <div class="title is-4">{ "Target" }</div>
                                    <table class="table">
                                        <thead>
                                            <tr>
                                                <th>{ "Block" }</th>
                                                if target.nonce.is_some() {
                                                <th>{ "Hex" }</th>
                                                <th>{ "Dec" }</th>
                                                }
                                            </tr>
                                        </thead>
                                        <tbody>
                                            <tr>
                                                <td class="is-family-monospace is-pulled-right">{ format!("{}", target.block.separate_with_commas()) }</td>
                                                if target.nonce.is_some() {
                                                <td class="is-family-monospace">{ format!("{:08X}", target.nonce.unwrap()) }</td>
                                                <td class="is-family-monospace is-pulled-right">{ format!("{}", target.nonce.unwrap().separate_with_commas()) }</td>
                                                }
                                            </tr>
                                        </tbody>
                                    </table>
                                </div>
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
                                    (*state).clone().guesses.unwrap_or_default().clone().into_iter().enumerate().map(|(pos, guess)| {
                                        html!{
                                        <tr key={ guess.name.clone() }>
                                            <th>{ &pos }</th>
                                            <td>{ &guess.name }</td>
                                            <td class="is-family-monospace">{ format!("{:08X}", &guess.nonce) }</td>
                                            <td class="is-family-monospace is-pulled-right">{ format!("{}", &guess.nonce.separate_with_commas()) }</td>
                                        </tr>
                                    }}).collect::<Html>()
                                }
                                </tbody>
                            </table>
                        </div>
                        <div class="control">
                            <button class = "button is-link" onclick={ onclick_update }>{"Update"}</button>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}
