document.addEventListener("DOMContentLoaded", function() {
    // The CommonMark parser wraps the math code in <code> tags with the class
    // `language-math`. It also adds a class `math-inline` to inline math and
    // `math-display` to display math.
    const mathCodeBlocks = document.querySelectorAll('code.language-math');

    mathCodeBlocks.forEach((codeBlock) => {
        const tex = codeBlock.textContent;

        const isDisplayMode = codeBlock.classList.contains('math-display');

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
