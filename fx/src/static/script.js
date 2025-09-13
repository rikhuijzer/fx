// Allow clicking on post previews to navigate to the post.
//
// This is a workaround for the fact that when wrapping the parent post in a
// href, then the text is no longer selectable. Also, nested links are not
// allowed in HTML. With this workaround, previews can still have nested links
// while also allowing clicking on non-link text to navigate to the post.
function make_post_previews_clickable() {
    const post_previews = document.querySelectorAll(".post-preview");
    post_previews.forEach((post_preview) => {
        post_preview.addEventListener("click", () => {
            if (window.getSelection().toString()) {
                // Don't navigate if the user has selected text.
                return;
            }
            const post_id = post_preview.getAttribute("data-post-link");
            window.location.href = `${post_id}`;
        });
        post_preview.style.cursor = "pointer";
    });
}

make_post_previews_clickable();

function reset_form_if_requested() {
    let urlParams = new URLSearchParams(window.location.search);
    if (urlParams.get("reset_forms") === "true") {
        const forms = document.querySelectorAll("form");
        forms.forEach((form) => {
            form.reset();
        });
    }
}

reset_form_if_requested();
