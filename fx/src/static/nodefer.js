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
        if (textareas[i].hasAttribute('required')) {
            disable_form_submit_if_empty(textareas[i]);
        }
    }
}

disable_form_submit_on_start();
