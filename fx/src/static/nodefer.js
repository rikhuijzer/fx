function copyCode(sha) {
    const code = document.getElementById('code-' + sha);
    const text = code.textContent;
    navigator.clipboard.writeText(text);

    const copy = document.getElementById('copy-' + sha);
    copy.textContent = 'copied';
    setTimeout(() => {
        copy.textContent = 'click to copy';
    }, 5000);
}
