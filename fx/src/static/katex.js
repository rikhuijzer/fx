document.addEventListener("DOMContentLoaded", function() {
    const mathCodeBlocks = document.querySelectorAll('code.language-math');

    mathCodeBlocks.forEach((codeBlock) => {
        const tex = codeBlock.textContent;

        const isDisplayMode = codeBlock.parentElement && codeBlock.parentElement.tagName === 'pre';

        const container = document.createElement(isDisplayMode ? 'div' : 'span');
        if (isDisplayMode) {
            container.classList.add('katex-display');
        }

        try {
            katex.render(tex, container, {
                throwOnError: false,
                displayMode: isDisplayMode
            });

            if (isDisplayMode && codeBlock.parentNode) {
                codeBlock.parentNode.parentNode.replaceChild(container, codeBlock.parentNode);
             } else if (codeBlock.parentNode){
                codeBlock.parentNode.replaceChild(container, codeBlock);
             }
        } catch (e) {
            console.error("KaTeX rendering error for:", tex, "\nError:", e);
        }
    });
});
