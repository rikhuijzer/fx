document.addEventListener("DOMContentLoaded", function() {
    // Select all code elements with the class language-math
    // We also check the parent to see if it's a <pre> block (indicating display math)
    const mathCodeBlocks = document.querySelectorAll('code.language-math');

    mathCodeBlocks.forEach((codeBlock) => {
        const tex = codeBlock.textContent; // Get the LaTeX content

        // Determine if it's display math based on the parent element
        // CommonMark parsers often put display math in <pre><code>...</code></pre>
        const isDisplayMode = codeBlock.parentElement && codeBlock.parentElement.tagName === 'PRE';

        // Create a container element to render the math into
        // Use <div> for display math, <span> for inline math
        const container = document.createElement(isDisplayMode ? 'div' : 'span');
         if (isDisplayMode) {
             container.classList.add('katex-display'); // Add KaTeX display class if needed for styling
         }

        try {
            // Render the math using katex.render()
            katex.render(tex, container, {
                throwOnError: false, // Set to true if you want to throw errors on invalid math
                displayMode: isDisplayMode
            });

            // Replace the original code block with the rendered container
            // For display math, replace the whole <pre> block
            if (isDisplayMode && codeBlock.parentNode) {
                codeBlock.parentNode.parentNode.replaceChild(container, codeBlock.parentNode);
             } else if (codeBlock.parentNode){
                // For inline math, replace just the <code> block
                codeBlock.parentNode.replaceChild(container, codeBlock);
             }


        } catch (e) {
            console.error("KaTeX rendering error for:", tex, "\nError:", e);
            // Optionally, handle the error visually, e.g., keep the original code block
            // or show an error message next to it.
            // Example: replace with an error message
             if (isDisplayMode && codeBlock.parentNode) {
                 codeBlock.parentNode.outerHTML = `<pre style="color:red;">Error rendering math: ${tex}</pre>`;
             } else {
                  codeBlock.outerHTML = `<code style="color:red;">Error rendering math: ${tex}</code>`;
             }
        }
    });
});
