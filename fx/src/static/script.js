function make_post_previews_clickable() {
    const post_previews = document.querySelectorAll(".post-preview");
    post_previews.forEach((post_preview) => {
        post_preview.addEventListener("click", () => {
            if (window.getSelection().toString()) {
                // Don't navigate if the user has selected text.
                return;
            }
            const post_id = post_preview.getAttribute("data-post-id");
            window.location.href = `/post/${post_id}`;

        });
        post_preview.style.cursor = "pointer";
    });
}

make_post_previews_clickable();
