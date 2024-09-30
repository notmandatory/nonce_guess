pub(crate) mod home;
pub(crate) mod login;

use maud::{html, Markup, DOCTYPE};

// base for all page templates
pub(crate) fn base(title: String, head: Option<Markup>, content: Markup) -> Markup {
    html! {
      (DOCTYPE)
      head {
        meta http-equiv="content-type" content="text/html; charset=UTF-8";
        meta charset="UTF-8";
        meta name="viewport" content="width=device-width, initial-scale=1.0";
        // TODO meta http-equiv="Content-Security-Policy" content="default-src 'self';
        link href="/assets/main.css" rel="stylesheet";
        link href="https://rsms.me/inter/inter.css" rel="stylesheet";
        link
          rel="apple-touch-icon"
          sizes="180x180"
          href="/assets/apple-touch-icon.png";
        link
          rel="icon"
          type="image/png"
          sizes="32x32"
          href="/assets/favicon-32x32.png";
        link
          rel="icon"
          type="image/png"
          sizes="16x16"
          href="/assets/favicon-16x16.png";
        link rel="manifest" href="/assets/site.webmanifest";
        title { (title) }
        script
          src="https://unpkg.com/htmx.org@1.9.6"
          integrity="sha384-FhXw7b6AlE/jyjlZH5iHa/tTe9EpJ1Y55RjcgPbjeWMskSxZt1v9qkxLJWNJaGni"
          crossorigin="anonymous" {}
        script src="https://unpkg.com/htmx.org/dist/ext/response-targets.js" {}
        script
          src="https://cdn.jsdelivr.net/npm/js-base64@3.7.4/base64.min.js"
          integrity="sha384-VkKbwLiG7C18stSGuvcw9W0BHk45Ba7P9LJG5c01Yo4BI6qhFoWSa9TQLNA6EOzI"
          crossorigin="anonymous" {}
        @if let Some(head) = head {
            (head)
        }
      }
      body ."min-h-screen"."bg-slate-50"."dark:bg-black"."dark:text-white"
          hx-boost="true" {
          main #"content"."max-w-4xl"."mx-auto" { (content) }
      }
    }
}
