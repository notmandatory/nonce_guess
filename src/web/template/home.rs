use super::base;
use crate::model::{Guess, Target};

pub fn home_page(
    target: Option<Target>,
    change_target: bool,
    my_guess: Option<u32>,
    guesses: Vec<Guess>,
) -> Markup {
    let content = html! {
     (hero_div())
     @if change_target {
         (change_target_div())
     }
     @if let Some(target) = target {
         (target_div(target))
     }
     @if my_guess.is_none() && !change_target  {
         (add_guess_div())
     }
     (guesses_div(guesses))
     (logout_div())
    };
    base("Nonce Guess".to_string(), None, content)
}

fn section(name: String, content: Markup) -> Markup {
    html! {
        section id=(name) ."flex"."flex-col"."justify-left"."p-6"."items-left"."gap-4"."scroll-mt-20" {
            (content)
        }
    }
}

pub fn logout_div() -> Markup {
    let content = html! {
        div ."gap-6" {
            button type="submit" hx-get="/logout" hx-push-url="/"
            ."rounded-md"."bg-indigo-600"."px-2.5"."py-1.5"."text-base"."font-semibold"."text-white"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-600"
            { "Logout" }
        }
    };
    section("logout".to_string(), content)
}

pub fn hero_div() -> Markup {
    html! {
        section #"hero"."flex"."flex-col"."justify-center"."sm:flex-row"."p-6"."items-center"."gap-8"."mb-12"."scroll-mt-20"{
            article ."sm:w-1/2" {
                h2 ."max-w-md"."text-4xl"."font-bold"."text-center"."sm:text-left"."text-slate-900"."dark:text-white" {
                    "Guess the Block Nonce"
                }
            }
            img ."h-20"."w-auto" src="../assets/apple-touch-icon.png" alt="Nonce Guess Logo";
        }
    }
}

fn guesses_div(guesses: Vec<Guess>) -> Markup {
    let content = html! {
        div ."flex"."items-center" {
            div ."sm:flex-auto" {
                h2 ."text-lg"."font-semibold"."leading-6"."text-grey-900" { "Current Guesses" }
            }
        }
        div {
            div {
                div ."inline-block"."min-w-full"."py-2"."align-middle"."sm:px-6"."lg:px-8" {
                    div ."overflow-hidden"."shadow"."ring-1"."ring-black"."ring-opacity-5"."sm:rounded-lg" {
                        table ."min-w-full"."divide-y"."divide-gray-300" {
                            thead ."bg-gray-50" {
                                tr {
                                    th scope="col"
                                        ."py-3.5"."pl-4"."pr-3"."text-left"."text-base"."font-semibold"."text-gray-900"."sm:pl-6" {
                                        "Position"
                                    }
                                    th scope="col"
                                        ."px-3"."py-3.5"."text-left"."text-base"."font-semibold"."text-gray-900" {
                                        "Name"
                                    }
                                    th scope="col"
                                        ."px-3"."py-3.5"."text-left"."text-base"."font-semibold"."text-gray-900" {
                                        "Hex"
                                    }
                                    th scope="col"
                                        ."px-3"."py-3.5"."text-left"."text-base"."font-semibold"."text-gray-900" {
                                        "Decimal"
                                    }
                                }
                            }
                            tbody ."divide-y"."divide-gray-200"."bg-white" {

                                @for guess in guesses.iter().enumerate() {
                                    tr {
                                        td ."whitespace-nowrap"."py-4"."pl-4"."pr-3"."text-base"."font-mono"."font-medium"."text-gray-900"."sm:pl-6" {
                                            (guess.0)
                                        }
                                        td ."whitespace-nowrap"."px-3"."py-4"."text-base"."text-gray-500" {
                                            (guess.1.name)
                                        }
                                        td ."whitespace-nowrap"."px-3"."py-4"."font-mono"."text-base"."text-gray-500" {
                                            ({format!("{:x}", guess.1.nonce)})
                                        }
                                        td ."whitespace-nowrap"."px-3"."py-4"."font-mono"."text-base"."text-gray-500" {
                                            (guess.1.nonce)
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    section("guesses".to_string(), content)
}

fn add_guess_div() -> Markup {
    let content = html! {
        div ."sm:flex"."sm:items-center" {
            div ."sm:flex-auto" {
                h2 ."text-lg"."font-semibold"."leading-6"."text-gray-900" { "Nonce Guess" }
            }
        }
        div ."mt-6"."flow-root" {
            div ."overflow-hidden"."shadow"."ring-1"."ring-black"."ring-opacity-5"."sm:rounded-lg" {
                div ."px-4"."sm:p-6" {
                    div ."sm:flex"."sm:items-start"."sm:justify-between" {
                        div {
                            div ."mt-2"."max-xl"."text-base"."text-gray-500" {
                                p { "Enter your nonce guess below. Your guess must be 8 hexadecimal characters which
                                    are a-f, A-F and 0-9.
                                    For example: `ab03f23e`, `f2345c9d`, etc. Every nonce in hex has a corresponding
                                    decimal representation, the guess that is closest numerically to the actual block nonce
                                    is the winner." }
                            }
                        }
                    }
                }

                div ."bg-white"."shadow-lg"."sm:rounded-lg" {
                    div ."px-4"."sm:p-6" {
                        div ."sm:flex"."sm:items-start"."sm:justify-between" {
                            form novalidate ."group" autocomplete="off" {
                                input type="text" name="guess" #"guess"
                                        required
                                        pattern="[0-9a-fA-F]{8}"
                                            ."block"."w-full"."rounded-md"."py-1.5"."text-gray-900"."shadow-sm"."ring-1"."ring-inset"."ring-gray-300"."placeholder:text-gray-400"."focus:ring-2"."focus:ring-inset"."focus:ring-indigo-600"."sm:text-base"."sm:leading-6"."peer"."invalid:[&:not(:placeholder-shown):not(:focus)]:border-red-500"
                                        placeholder=" ";
                                div ."hidden"."py-1.5"."leading-6"."gap-6"."text-red-600"."font-semibold"."peer-[&:not(:placeholder-shown):not(:focus):invalid]:block" {
                                    p #"error_message" {
                                        "Must be 8 characters and only include a-f, A-F, and 0-9."
                                    }
                                }
                                div ."py-1.5" {
                                    button type="submit"
                                        hx-post="/"
                                        hx-target="body"
                                        hx-target-5xx="#error_message"
                                        hx-trigger="click, keyup[key=Enter]"
                                            ."inline-flex"."py-1.5"."items-center"."rounded-md"."bg-indigo-600"."px-3"."py-2"."text-base"."font-semibold"."text-white"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-500"."group-invalid:pointer-events-none"."group-invalid:opacity-30"{
                                        "Add Guess"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    section("add_guess".to_string(), content)
}

fn change_target_div() -> Markup {
    let content = html! {
        div ."flex"."items-center" {
            div ."flex-auto" {
                h2 ."text-lg"."font-semibold"."leading-6"."text-gray-900" { "Change Target" }
            }
        }
        div {
            div {
                div ."inline-block"."min-w-full"."py-2"."align-middle"."sm:px-6"."lg:px-8" {
                    div ."max-w-md"."divide-y"."divide-gray-300" {
                        div ."flex"."flex-col"."max-w-small"."justify-left"."items-left" {
                            form novalidate ."group"."max" autocomplete="off" {
                                input type="text" name="block" #"block"
                                        required
                                        pattern="[0-9]{6}"
                                            ."block"."w-full"."rounded-md"."py-1.5"."text-gray-900"."shadow-sm"."ring-1"."ring-inset"."ring-gray-300"."placeholder:text-gray-400"."focus:ring-2"."focus:ring-inset"."focus:ring-indigo-600"."sm:text-base"."sm:leading-6"."peer"."invalid:[&:not(:placeholder-shown):not(:focus)]:border-red-500"
                                        placeholder=" ";
                                div ."hidden"."py-1.5"."leading-6"."gap-6"."text-red-600"."font-semibold"."peer-[&:not(:placeholder-shown):not(:focus):invalid]:block" {
                                    p #"error_message" {
                                        "Must be valid block number."
                                    }
                                }
                                div ."py-1.5" {
                                    button type="submit"
                                        hx-post="/target"
                                        hx-target="body"
                                        hx-target-5xx="#error_message"
                                        hx-trigger="click, keyup[key=Enter]"
                                            ."inline-flex"."py-1.5"."items-center"."rounded-md"."bg-indigo-600"."px-3"."py-2"."text-base"."font-semibold"."text-white"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-500"."group-invalid:pointer-events-none"."group-invalid:opacity-30"{
                                        "Set Block"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    section("change_target".to_string(), content)
}

fn target_div(target: Target) -> Markup {
    let content = html! {
        div ."flex"."items-center" {
            div ."flex-auto" {
                h2 ."text-lg"."font-semibold"."leading-6"."text-gray-900" { "Target" }
            }
        }

        div {
            div {
                div ."inline-block"."min-w-full"."py-2"."align-middle"."sm:px-6"."lg:px-8" {
                    div ."overflow-hidden"."shadow"."ring-1"."ring-black"."ring-opacity-5"."sm:rounded-lg" {
                        table ."min-w-full"."divide-y"."divide-gray-300" {
                            thead ."bg-gray-50" {
                                tr {
                                    th scope="col"
                                        ."py-3.5"."pl-4"."pr-3"."text-left"."text-base"."font-semibold"."text-gray-900"."sm:pl-6" {
                                        "Block"
                                    }
                                    th scope="col"
                                        ."px-3"."py-3.5"."text-left"."text-base"."font-semibold"."text-gray-900" {
                                        "Hex"
                                    }
                                    th scope="col"
                                        ."px-3"."py-3.5"."text-left"."text-base"."font-semibold"."text-gray-900" {
                                        "Decimal"
                                    }
                                }
                            }
                            tbody ."divide-y"."divide-gray-200"."bg-white" {
                                tr {
                                    td ."whitespace-nowrap"."py-4"."pl-4"."pr-3"."font-mono"."text-base"."font-medium"."text-gray-900"."sm:pl-6" {
                                        (target.block)
                                    }
                                    td ."whitespace-nowrap"."px-3"."py-4"."font-mono"."text-base"."text-gray-500" {
                                        @if let Some(nonce) = target.nonce {
                                            ({format!("{:x}", nonce)})
                                        }
                                        @else {
                                            "TBD"
                                        }
                                    }
                                    td ."whitespace-nowrap"."px-3"."py-4"."font-mono"."text-base"."text-gray-500" {
                                        @if let Some(nonce) = target.nonce {
                                            (nonce)
                                        }
                                        @else{
                                            "TBD"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    };
    section("target".to_string(), content)
}
