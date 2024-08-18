use maud::{html, Markup};

use super::base;

// login page template
pub fn login_page(next: Option<String>) -> Markup {
    let head = html! {
        script src="assets/auth.js" async="true" {}
    };
    let content = html! {
        div .flex."min-h-full"."flex-col"."justify-center"."px-6"."py-12"."lg:px-8" {
            div ."sm:mx-auto"."sm:w-full"."sm:max-w-sm" {
                img ."mx-auto"."h-20"."w-auto" src="../assets/apple-touch-icon.png" alt="Nonce Guess";
                h2 ."mt-10"."text-center"."text-2xl"."font-bold"."leading-9"."tracking-tight"."text-gray-900" {
                    "Sign in to guess the block nonce"
                }
            }

            div ."mt-10"."sm:mx-auto"."sm:w-full"."sm:max-w-sm" {
                form ."group"."space-y-6" action="#" method="POST" {
                    div {
                        label ."block"."text-sm"."font-medium"."leading-6"."text-gray-900" for="username" { "Name" }
                        div ."mt-2" {
                            input id="username" name="username" type="text" autocomplete="username webauthn" required placeholder=" "
                            ."block"."w-full"."rounded-md"."border-0"."py-1.5"."text-gray-900"."shadow-sm"."ring-1"."ring-inset"."ring-gray-300"."placeholder:text-gray-400"."focus:ring-2"."focus:ring-inset"."focus:ring-indigo-600"."sm:text-sm"."sm:leading-6";
                        }
                    }

                    div ."mt-6"."flex"."items-center"."justify-end"."gap-x-6" {
                        button type="submit" onclick="login()"
                        ."flex"."w-full"."justify-center"."rounded-md"."bg-indigo-600"."px-3"."py-1.5"."text-sm"."font-semibold"."leading-6"."text-white"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-600"."group-invalid:pointer-events-none"."group-invalid:opacity-30" {
                            "Sign in"
                        }
                        button type="submit" onclick="register()"
                        ."flex"."w-full"."justify-center"."rounded-md"."px-3"."py-1.5"."text-sm"."font-semibold"."leading-6"."text-gray-900"."shadow-sm"."hover:bg-gray-100"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."group-invalid:pointer-events-none"."group-invalid:opacity-30" {
                            "Register"
                        }
                    }
                    div ."py-1.5"."leading-6"."gap-6"."text-green-600"."font-semibold" {
                        p id="flash_message";
                    }
                    @if  let Some(next) = next {
                        input type="hidden" name="next" value=(next);
                    }
                }

                p ."mt-10"."text-center"."text-sm"."text-gray-500" {
                    "No account? register with your "
                    a ."font-semibold"."leading-6"."text-indigo-600"."hover:text-indigo-500"
                        href="https://fidoalliance.org/passkeys/" target="_blank" rel="noopener noreferrer" {
                            "Passkey"
                    }
                    " enabled browser."
                }
            }
        }
    };
    base("Login".to_string(), Some(head), content)
}
