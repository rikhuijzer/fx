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
