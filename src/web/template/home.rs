use maud::{html, Markup};

use super::base;
use crate::model::{Guess, Target};

pub fn home_page(
    target: Option<Target>,
    change_target: bool,
    my_guess: Option<u32>,
    guesses: Vec<Guess>,
) -> Markup {
    let content = html! {
        div .flex."min-h-full"."flex-col"."justify-center"."px-6"."py-12"."lg:px-8" {
            div ."sm:mx-auto"."sm:w-full"."py-6"."md:max-w-4xl" {
                div ."py-6" {
                    h1 ."text-4xl"."font-bold" {
                        img src="../assets/apple-touch-icon.png" width="75" height="75";
                        br;
                        "Guess the Block Nonce"
                    }
                }
            }


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

            div ."sm:mx-auto"."sm:w-full"."py-6"."divide-y"."md:max-w-4xl" {
                div "mt-8"."gap-6" {
                    button
                    type="submit"
                    hx-get="/logout"
                    hx-push-url="/"
                        ."rounded-md"."bg-indigo-600"."px-2.5"."py-1.5"."text-base"."font-semibold"."text-white"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-600"
                    {
                    "Logout"
                    }
                }
            }
        }
    };
    base("Nonce Guess".to_string(), None, content)
}

fn guesses_div(guesses: Vec<Guess>) -> Markup {
    html! {
        div ."sm:mx-auto"."sm:w-full"."py-6"."divide-y"."md:max-w-4xl" {
            div ."sm:flex"."sm:items-center" {
                div ."sm:flex-auto" {
                    h2 ."text-lg"."font-semibold"."leading-6"."text-gray-900" "Current Guesses";
                }
            }
            div ."mt-6"."flow-root" {
                div ."-mx-4"."-my-2"."overflow-x-auto"."sm:-mx-6"."lg:-mx-8" {
                    div ."inline-block"."min-w-full"."py-2"."align-middle"."sm:px-6"."lg:px-8" {
                        div ."overflow-hidden"."shadow ring-1"."ring-black"."ring-opacity-5"."sm:rounded-lg" {
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
        }
    }
}

fn add_guess_div() -> Markup {
    html! {
        div ."sm:mx-auto"."sm:w-full"."py-6"."divide-y"."md:max-w-4xl" {
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
                                        For example: "ab03f23e", "f2345c9d", etc. Every nonce in hex has a corresponding
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
                                        button type="button"
                                                hx-post="/"
                                                hx-target="body"
                                                hx-target-5xx="#error_message"
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
        }
    }
}

fn change_target_div() -> Markup {
    html! {
        div ."sm:mx-auto"."sm:w-full"."py-6"."divide-y"."md:max-w-4xl" {
            div ."sm:flex"."sm:items-center" {
                div ."sm:flex-auto" {
                    h2 ."text-lg"."font-semibold"."text-gray-900" { "Change Target Block" }
                }
            }
            div ."mt-6"."flow-root" {
                div ."overflow-hidden"."shadow"."ring-1"."ring-black"."ring-opacity-5"."sm:rounded-lg" {
                    div ."bg-white"."shadow-lg"."sm:rounded-lg" {
                        div ."px-4"."sm:p-6" {
                            div ."sm:flex"."sm:items-start"."sm:justify-between" {
                                form novalidate ."group" autocomplete="off" {
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
                                        button type="button"
                                                hx-post="/target"
                                                hx-target="body"
                                                hx-target-5xx="#error_message"
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
        }
    }
}

fn target_div(target: Target) -> Markup {
    html! {
        div ."sm:mx-auto"."sm:w-full"."py-6"."divide-y"."md:max-w-4xl" {
            div ."sm:flex"."sm:items-center" {
                div ."sm:flex-auto" {
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
        }
    }
}
