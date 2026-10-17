function showCopied(id) {
    const copy = document.getElementById(id);
    copy.textContent = 'copied';
    setTimeout(() => {
        copy.textContent = 'copy';
    }, 5000);
}

function copyCode(sha) {
    const code = document.getElementById('code-' + sha);
    const text = code.textContent;
    navigator.clipboard.writeText(text);
    showCopied('copy-' + sha);
}

function copyLongUrl() {
    const slug = document.getElementById('long-url').href;
    navigator.clipboard.writeText(slug);
    showCopied('copy-long-url');
}

function disable_form_submit_if_empty(textarea) {
    const form = textarea.closest('form');
    const submitButtons = form.querySelectorAll('input[type="submit"]');
    const hasContent = 0 < textarea.value.trim().length;

    submitButtons.forEach((button) => {
        button.disabled = !hasContent;
    });
}

function disable_form_submit_on_start() {
    const textareas = document.getElementsByTagName('textarea');
    for (let i = 0; i < textareas.length; i++) {
        const textarea = textareas[i];
        const form = textarea.closest('form');
        // Limit this to post forms so that an empty settings textarea does not
        // disable the settings save button.
        const isPostForm = textarea.hasAttribute('required') ||
            form.querySelector('input[name="preview"]') !== null;
        if (isPostForm) {
            disable_form_submit_if_empty(textarea);
        }
    }
}

disable_form_submit_on_start();

// Chromium can restore disabled submit buttons from the back-forward cache
// after showing a post preview. Recalculate them from the restored textarea.
window.addEventListener('pageshow', disable_form_submit_on_start);
