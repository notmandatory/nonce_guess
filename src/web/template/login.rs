use maud::{html, Markup};

use super::base;

// login page template
pub fn login_page() -> Markup {
    let head = html! {
        script src="assets/auth.js" async="true" {}
    };
    // let content = html! {
    //     div ."flex"."min-h-full"."flex-col"."justify-center"."px-6"."py-12"."lg:px-8" {
    //         div ."sm:mx-auto"."sm:w-full"."sm:max-w-sm" {
    //             img ."mx-auto"."h-20"."w-auto" src="../assets/apple-touch-icon.png" alt="Nonce Guess";
    //             h2 ."mt-10"."text-center"."text-2xl"."font-bold"."leading-9"."tracking-tight"."text-gray-900" {
    //                 "Sign in to guess the block nonce"
    //             }
    //         }
    //
    //         div ."mt-10"."sm:mx-auto"."sm:w-full"."sm:max-w-sm" {
    //             form ."group"."space-y-6" action="#" method="POST" {
    //                 div {
    //                     label ."block"."text-sm"."font-medium"."leading-6"."text-gray-900" for="username" { "Name" }
    //                     div ."mt-2" {
    //                         input #username name="username" type="text" autocomplete="username webauthn" required placeholder=" "
    //                         ."block"."w-full"."rounded-md"."border-0"."py-1.5"."text-gray-900"."shadow-sm"."ring-1"."ring-inset"."ring-gray-300"."placeholder:text-gray-400"."focus:ring-2"."focus:ring-inset"."focus:ring-indigo-600"."sm:text-sm"."sm:leading-6";
    //                     }
    //                 }
    //
    //                 div ."mt-6"."flex"."items-center"."justify-end"."gap-x-6" {
    //                     button type="submit" onclick="login()"
    //                     ."flex"."w-full"."justify-center"."rounded-md"."bg-indigo-600"."px-3"."py-1.5"."text-sm"."font-semibold"."leading-6"."text-white"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-600"."group-invalid:pointer-events-none"."group-invalid:opacity-30" {
    //                         "Sign in"
    //                     }
    //                     button type="submit" onclick="register()"
    //                     ."flex"."w-full"."justify-center"."rounded-md"."px-3"."py-1.5"."text-sm"."font-semibold"."leading-6"."text-gray-900"."shadow-sm"."hover:bg-gray-100"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."group-invalid:pointer-events-none"."group-invalid:opacity-30" {
    //                         "Register"
    //                     }
    //                 }
    //                 div ."py-1.5"."leading-6"."gap-6"."text-green-600"."font-semibold" {
    //                     p #"flash_message";
    //                 }
    //             }
    //
    //             p ."mt-10"."text-center"."text-sm"."text-gray-500" {
    //                 "No account? register with your "
    //                 a ."font-semibold"."leading-6"."text-indigo-600"."hover:text-indigo-500"
    //                     href="https://fidoalliance.org/passkeys/" target="_blank" rel="noopener noreferrer" {
    //                         "Passkey"
    //                 }
    //                 " enabled browser."
    //             }
    //         }
    //     }
    // };
    let content = html! {
        section #"hero"."flex"."flex-col-reverse"."justify-center"."sm:flex-row"."p-6"."items-center"."gap-8"."mb-12"."scroll-mt-20" {
            article ."sm:w-1/2" {
                h2 ."max-w-md"."text-4xl"."font-bold"."text-center"."sm:text-left"."text-slate-900"."dark:text-white" {
                        "Guess the "
                        span ."text-indigo-700"."dark:text-indigo-300" { "Block Nonce " }
                        "Win a Prize!"
                }
            }
            img ."h-20"."w-auto" src="../assets/apple-touch-icon.png" alt="Nonce Guess Logo";
        }
        section #"login_form"."flex"."flex-col"."justify-center"."p-6"."items-center"."gap-4"."scroll-mt-20" {
                form ."group" novalidate {
                    label ."text-left"."text-xl"."font-bold"."text-slate-900"."dark:text-white" for="username"
                    { "Name" }
                    input
                        #"username"."block"."mt-2"."w-60"."rounded-md"."py-1.5"."dark:bg-gray-900"."dark:text-white"."text-gray-900"."shadow-sm"."ring-inset"."ring-gray-300"."dark:ring-black"."placeholder:text-gray-400"."focus:ring-2"."focus:ring-inset"."focus:ring-indigo-600"."sm:text-sm"."sm:leading-6"."peer"."invalid:[&:not(:placeholder-shown):not(:focus)]:border-red-500"
                        name="username"
                        type="text"
                        autocomplete="username webauthn"
                        required=""
                        placeholder=" "
                        pattern="[0-9a-zA-Z_]{3,20}";
                    div ."hidden"."w-60"."py-1.5"."leading-6"."gap-6"."text-red-600"."font-semibold"."peer-[&:not(:placeholder-shown):not(:focus):invalid]:block" {
                        p #"error_message" {
                            "Must be 3-20 characters and only include A-Z, 0-9, and underscore."
                        }
                    }
                    div ."mt-6"."flex"."items-center"."justify-center"."gap-x-6" {
                        button
                            ."flex"."justify-center"."rounded-md"."bg-indigo-600"."px-3"."py-1.5"."text-sm"."font-semibold"."leading-6"."text-gray-100"."shadow-sm"."hover:bg-indigo-500"."focus-visible:outline"."focus-visible:outline-2"."focus-visible:outline-offset-2"."focus-visible:outline-indigo-600"."group-invalid:pointer-events-none"."group-invalid:opacity-30"
                            type="submit"
                            hx-trigger="click[enterKey]"
                            onclick="login()"
                        { "Sign in" }
                        button
                            ."flex"."justify-center"."rounded-md"."bg-indigo-300"."px-3"."py-1.5"."text-sm"."font-semibold"."leading-6"."text-gray-900"."dark:hover:bg-indigo-200"."shadow-sm"."hover:bg-indigo-200"."focus-visible:outline"."dark:bg-indigo-300"."focus-visible:outline-2"."focus-visible:outline-offset-2"."group-invalid:pointer-events-none"."group-invalid:opacity-30"
                            type="submit"
                            onclick="register()"
                        { "Register" }
                    }
                    div ."py-1.5"."leading-6"."gap-6"."text-green-600"."font-semibold" {
                        p #"flash_message";
                    }
                }
                p ."mt-5"."text-center"."text-sm"."text-gray-500" {
                    "No account? register with your "
                    a ."font-semibold"."leading-6"."text-indigo-600"."hover:text-indigo-500"
                        href="https://fidoalliance.org/passkeys/"
                        target="_blank"
                        rel="noopener noreferrer"
                        { "Passkey " }
                    "enabled browser."
                }
        }
    };
    base("Login".to_string(), Some(head), content)
}
