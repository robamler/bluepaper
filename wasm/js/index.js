const wasm = import("../pkg/index.js");
var accessToken = null;


function makeFormatter() {
    var output = `\\documentclass{article}

\\usepackage[utf8]{inputenc}
\\usepackage{amssymb,amsmath,amsfonts}
\\usepackage[normalem]{ulem}
\\usepackage{graphicx}
\\usepackage[unicode=true]{hyperref}

\\newcommand{\\checkedbox}{\\mbox{\\ooalign{$\\square$\\cr\\hidewidth\\raisebox{.45ex}{\\hspace{0.2em}$\\checkmark$}\\hidewidth\\cr}}}
\\newcommand{\\uncheckedbox}{$\\square$}

\\begin{document}

`;
    var indentation = 0;
    var newlines = 0;
    var maxNewlines = Infinity;

    function writeUnescaped(str) {
        const n = Math.min(newlines, maxNewlines);
        if (n != 0) {
            if (str.match(/^\s*$/)) {
                // Don't write whitespace-only strings at the beginning of a line.
                return;
            }
            output += "\n".repeat(n) + "  ".repeat(indentation);
        }
        newlines = 0;
        maxNewlines = Infinity;
        output += str;
    }

    var writeEscaped = writeUnescaped; // TODO

    function addNewlines(num) {
        newlines = Math.max(num, newlines);
    }

    function limitNewlines(num) {
        maxNewlines = num;
    }

    function indent() {
        indentation += 1;
    }

    function unindent() {
        if (indentation > 0) {
            indentation -= 1;
        }
    }

    function finish() {
        addNewlines(2);
        writeUnescaped("\\end{document}\n")
        return output;
    }

    return {
        writeUnescaped,
        writeEscaped,
        addNewlines,
        limitNewlines,
        indent,
        unindent,
        finish
    }
}


const Converter = (function () {
    var queue = {};
    var fileNames = [];
    var generation = 0;

    const LATEX_LISTS = {
        "bullet": "itemize",
        "number": "enumerate",
        "quote": "quote",
        "task": "itemize",
        "taskdone": "itemize",
        "indent": "indent",
    };

    function clearQueue() {
        queue = {};
        fileNames = [];
        generation += 1;
    }

    var foundImgUrl = function (wasm_module) {
        return function (url) {
            if (!queue.hasOwnProperty(url)) {
                const match = url.match(/_([^_]+?_*)\.(png|jpg|jpeg|svg)$/i);
                if (match) {
                    const basename = match[1];
                    var suffix = "." + match[2].toLowerCase();
                    const isSvg = suffix === ".svg"
                    if (isSvg) {
                        suffix = ".png";
                    }

                    var fileName = basename + suffix;
                    var i = 2;
                    while (fileNames.indexOf(fileName) >= 0) {
                        fileName = basename + "-" + i + suffix;
                        i += 1;
                    }
                    fileNames.push(fileName);

                    const originalGeneration = generation;
                    var promise = fetchPolyFill(url, "GET", "arraybuffer").then(
                        xhr => xhr.response);
                    if (isSvg) {
                        promise = promise.then(buf => {
                            if (generation === originalGeneration) {
                                return svgToPng(buf);
                            }
                        });
                    }
                    queue[url] = promise.then(buf => {
                        if (generation === originalGeneration) {
                            wasm_module.register_image(url, fileName, new Uint8Array(buf));
                        }
                    });
                }
            }
        };
    };

    function generateLatex(input, inputFormat, wasm_module) {
        clearQueue();
        try {
            URL.revokeObjectURL(document.getElementById("save-zip").href);
        } catch (e) { }

        var latex;
        if (inputFormat == "markdown") {
            latex = wasm_module.markdown_to_latex(input, foundImgUrl(wasm_module));
        } else {
            wasm_module.clear_registered_images();
            latex = htmlToLatex(input, wasm_module, false);
        }

        if (fileNames.length !== 0) {
            document.getElementById("wait-zip").style.display = "inline";
            const originalGeneration = generation;
            Promise.all(Object.values(queue)).then(function () {
                if (generation === originalGeneration) {
                    var zipFileData;
                    if (inputFormat == "markdown") {
                        zipFileData = wasm_module.markdown_to_zipped_latex(input);
                    } else {
                        latex = htmlToLatex(input, null, true);
                        zipFileData = wasm_module.latex_to_zipped_latex(latex);
                    }

                    const blob = new Blob([zipFileData], { type: "application/zip" });
                    const url = window.URL.createObjectURL(blob);
                    document.getElementById("save-zip").href = url;
                    document.getElementById("wait-zip").style.display = "none";
                    document.getElementById("save-zip-container").style.display = "inline";
                }
            });
        } else {
            document.getElementById("wait-zip").style.display = "none";
        }

        return latex;
    }

    function htmlToLatex(html, wasm_module, uncommentGraphics) {
        function processDivList(div) {
            var listTypesStack = ["indent"];
            var listLevelsStack = [0];
            var inCodeBlock = false;

            while (div) {
                const child = div.firstElementChild;
                const isCodeBlockLine = child && child.tagName === "CODE";
                const isListItem = child && child.tagName === "UL" || child.tagName === "OL";

                if (!isCodeBlockLine && inCodeBlock) {
                    inCodeBlock = false;
                    formatter.writeUnescaped("\\end{verbatim}");
                    formatter.addNewlines(2);
                }

                if (!isListItem) {
                    while (listLevelsStack[listLevelsStack.length - 1] !== 0) {
                        formatter.unindent();
                        formatter.unindent();
                        formatter.addNewlines(1);
                        formatter.limitNewlines(1);
                        formatter.writeUnescaped("\\end{" + listTypesStack.pop() + "}");
                        formatter.addNewlines(2);
                        listLevelsStack.pop();
                    }
                }

                if (isCodeBlockLine) {
                    if (!inCodeBlock) {
                        inCodeBlock = true;
                        formatter.addNewlines(2);
                        formatter.writeUnescaped("\\begin{verbatim}\n");
                    }
                    formatter.writeEscaped(child.textContent);
                    formatter.addNewlines(1);
                }

                if (isListItem) {
                    var listLevel = null;
                    var originalListType = null;
                    child.classList.forEach(c => {
                        var m = c.match(/^listindent(\d+)$/);
                        if (m) {
                            listLevel = parseInt(m[1]);
                        }
                        var m = c.match(/^listtype-(indent|bullet|number|quote|task|taskdone)$/);
                        if (m) {
                            originalListType = m[1];
                        }
                    });
                    const listType = LATEX_LISTS[originalListType];

                    if (listLevel && listType) {
                        while (listLevel < listLevelsStack[listLevelsStack.length - 1]
                            || (listType !== "indent"
                                && listLevel === listLevelsStack[listLevelsStack.length - 1]
                                && listTypesStack[listTypesStack.length - 1] !== listType)) {
                            formatter.unindent();
                            const endListType = listTypesStack.pop();
                            if (endListType !== "quote") {
                                formatter.unindent();
                            }
                            formatter.addNewlines(1);
                            formatter.limitNewlines(1);
                            formatter.writeUnescaped("\\end{" + endListType + "}");
                            formatter.addNewlines(2);
                            listLevelsStack.pop();
                        }

                        if (listType !== "indent") {
                            if (listLevel > listLevelsStack[listLevelsStack.length - 1]) {
                                listTypesStack.push(listType);
                                listLevelsStack.push(listLevel);
                                if (listLevel === 1) {
                                    formatter.addNewlines(2);
                                } else {
                                    formatter.addNewlines(1);
                                }
                                formatter.writeUnescaped("\\begin{" + listType + "}");
                                formatter.addNewlines(1);
                                formatter.limitNewlines(1);
                                formatter.indent();
                                if (listType !== "quote") {
                                    formatter.indent();
                                }
                            }

                            if (listType !== "quote") {
                                formatter.addNewlines(2);
                                formatter.unindent();
                                if (originalListType === "task") {
                                    formatter.writeUnescaped("\\item[\\uncheckedbox] ");
                                } else if (originalListType === "taskdone") {
                                    formatter.writeUnescaped("\\item[\\checkedbox] ");
                                } else {
                                    formatter.writeUnescaped("\\item ");
                                }
                                formatter.indent();
                                formatter.limitNewlines(0);
                            } else {
                                formatter.addNewlines(1);
                            }
                        }
                    }
                }

                if (!isCodeBlockLine) {
                    if (div.firstElementChild && div.firstElementChild.classList.contains("ace-separator")) {
                        formatter.addNewlines(2);
                        formatter.writeUnescaped("\\medbreak\\hrule\\medbreak");
                        formatter.addNewlines(2);
                    } else if (div.textContent === "" && (
                        (div.firstElementChild && div.firstElementChild.tagName === "BR")
                        || listTypesStack[listTypesStack.length - 1] == "quote")) {
                        // Empty line, signalling a new paragraph.
                        formatter.addNewlines(2);
                    } else {
                        formatter.addNewlines(1);
                        processChildren(div);
                    }
                }

                div = div.nextElementSibling;
            }
        }

        function processChildren(element) {
            for (var child = element.firstChild; child; child = child.nextSibling) {
                if (child.nodeType === Node.TEXT_NODE) {
                    formatter.writeEscaped(child.textContent);
                } else if (child.nodeType === Node.ELEMENT_NODE) {
                    processElement(child);
                }
            }
        }

        function processElement(el) {
            if (el.tagName === "H1") {
                formatter.addNewlines(3);
                formatter.writeUnescaped("\\section{")
                processChildren(el);
                formatter.writeUnescaped("}");
                formatter.addNewlines(2);
                formatter.limitNewlines(2);
            } else if (el.tagName === "H2") {
                formatter.addNewlines(3);
                formatter.writeUnescaped("\\subsection{")
                processChildren(el);
                formatter.writeUnescaped("}");
                formatter.addNewlines(2);
                formatter.limitNewlines(2);
            } else if (el.tagName === "SPAN") {
                if (el.classList.contains("ace-all-bold-hthree")) {
                    formatter.addNewlines(2);
                    formatter.writeUnescaped("\\paragraph{");
                    processChildren(el.firstElementChild.firstElementChild);
                    formatter.writeUnescaped("}");
                    formatter.addNewlines(1);
                    formatter.limitNewlines(2);
                } else if (el.classList.contains("inline-code")) {
                    formatter.writeUnescaped("\\texttt{");
                    formatter.writeEscaped(el.textContent);
                    formatter.writeUnescaped("}");
                } else if (el.classList.contains("inline-latex")) {
                    formatter.writeUnescaped("$");
                    formatter.writeUnescaped(el.getAttribute("data-current-latex-value"));
                    formatter.writeUnescaped("$");
                } else if (!el.hasAttribute("data-faketext")) {
                    processChildren(el);
                }
            } else if (el.tagName === "I") {
                formatter.writeUnescaped("\\emph{");
                processChildren(el);
                formatter.writeUnescaped("}");
            } else if (el.tagName === "B") {
                formatter.writeUnescaped("\\textbf{");
                processChildren(el);
                formatter.writeUnescaped("}");
            } else if (el.tagName === "S") {
                formatter.writeUnescaped("\\sout{");
                processChildren(el);
                formatter.writeUnescaped("}");
            } else if (el.tagName === "A") {
                formatter.writeUnescaped("\\href{");
                formatter.writeEscaped(el.href);
                formatter.writeUnescaped("}{");
                processChildren(el);
                formatter.writeUnescaped("}");
            } else if (el.tagName === "IMG") {
                numImages += 1;
                const fileName = "figure-" + numImages + ".png";
                if (wasm_module) {
                    fileNames.push(fileName);
                    queue[fileName] = fetchPolyFill(el.src, "GET", "arraybuffer").then(xhr => {
                        wasm_module.register_image(fileName, "", new Uint8Array(xhr.response));
                    });
                }
                formatter.addNewlines(2);
                formatter.writeUnescaped(
                    (uncommentGraphics ? "" : "%") +
                    "\\includegraphics[width=\\textwidth]{figures/" + fileName + "}");
                formatter.addNewlines(2);
            } else if (el.tagName === "TABLE") {
                var colCount = el.firstElementChild.firstElementChild.childElementCount;
                formatter.addNewlines(2);
                formatter.writeUnescaped("\\begin{tabular}{" + "l".repeat(colCount) + "}");
                formatter.addNewlines(1);
                formatter.indent();

                for (var tr = el.firstElementChild.firstElementChild; tr; tr = tr.nextElementSibling) {
                    for (var td = tr.firstElementChild; td; td = td.nextElementSibling) {
                        if (td !== tr.firstElementChild) {
                            formatter.writeUnescaped("& ");
                        }
                        formatter.indent();
                        processChildren(td);
                        formatter.unindent();
                        formatter.addNewlines(1);
                    }

                    if (tr.nextElementSibling) {
                        if (tr === el.firstElementChild.firstElementChild) {
                            formatter.writeUnescaped("\\\\\\hline");
                        } else {
                            formatter.writeUnescaped("\\\\");
                        }
                    }
                    formatter.addNewlines(1);
                }

                formatter.unindent();
                formatter.writeUnescaped("\\end{tabular}");
                formatter.addNewlines(2);
            } else {
                processChildren(el);
            }
        }

        var formatter = makeFormatter();
        var listIndent = 0;
        var numImages = 0;

        const body = (new DOMParser()).parseFromString(html, 'text/html')
            .documentElement
            .getElementsByTagName("body")[0];
        const title = body.querySelector(".hp-print-mode .ace-feature-bigtitle > .ace-editor > div:first-child");
        formatter.writeUnescaped("\\title{")
        processChildren(title);
        formatter.writeUnescaped("}\n\\maketitle");
        formatter.addNewlines(2);
        formatter.limitNewlines(2);

        processDivList(title.nextElementSibling);

        return formatter.finish();
    }

    return {
        generateLatex,
    }
}())

function onDomContentLoaded() {
    window.onhashchange = handleHashChange;
    document.getElementById("authorize").addEventListener("click", startAuthorization);
    document.getElementById("latex").addEventListener("click", selectAllLatex);
    document.getElementById("copy").addEventListener("click", copyLatex);
    document.getElementById("save-zip").addEventListener("click", saveZip);
    document.getElementById("back-to-start").addEventListener("click", backToStart);
    document.body.addEventListener('dragover', handleDragOver, false);
    document.body.addEventListener('dragexit', handleDragLeave, false);
    document.body.addEventListener('drop', handleFileDrop, false);
    document.getElementById("files").addEventListener("change", handleFileSelected);

    const match = document.cookie.match(/(^|; ?)bluepaper_dropbox_access_tokens=([^;]+)/);
    if (match) {
        // Update the cookie's expiration date.
        document.cookie = (
            "bluepaper_dropbox_access_tokens=" + match[2]
            + ";max-age=31536000;secure;samesite=lax"
        );

        const savedTokens = JSON.parse(decodeURIComponent(match[2]));
        accessToken = savedTokens.tokens[savedTokens.last_used];
        getPaperDocs();
    }
}

function svgToPng(svgArrayBuffer) {
    return new Promise((resolve, reject) => {
        var canvas = document.createElement("canvas");

        var img = new Image();
        img.style = "max-width:2000px;max-height:3000px"; // TODO: test this.
        img.onload = function () {
            // TODO: scale up to maximum resolution (need to find out how to do that
            //       before the image is converged to a bitmap.)
            canvas.setAttribute("width", "" + img.width);
            canvas.setAttribute("height", "" + img.height);
            canvas.getContext('2d').drawImage(img, 0, 0, img.width, img.height);
            URL.revokeObjectURL(img.src);
            canvas.toBlob(blob => {
                const reader = new FileReader();
                reader.onload = e => {
                    resolve(e.target.result);
                };
                reader.readAsArrayBuffer(blob);
            }, "image/png");
        };

        const svgBlob = new Blob([svgArrayBuffer], { type: "image/svg+xml" });
        img.src = window.URL.createObjectURL(svgBlob);
    });
}

const startAuthorization = (function () {
    var windowObjectReference = null;

    return function (e) {
        e.preventDefault();

        const remember = !!document.getElementById("remember").checked;
        const url = (
            "https://www.dropbox.com/oauth2/authorize?client_id=ebapspmlhsl9rn9&response_type=token&redirect_uri=https%3A%2F%2Frobamler.github.io%2Fbluepaper%2Foauth-redirect.html"
            + "&state=remember%3D" + remember
        );

        if (windowObjectReference === null || windowObjectReference.closed) {
            windowObjectReference = window.open(
                url,
                "authorize",
                "width=730,height=700,location,status,resizable,scrollbars"
            );
            if (windowObjectReference === null) {
                location.href = url;
            }
        } else {
            windowObjectReference.focus();
        }
    };
}());

function handleDragOver(e) {
    const items = e.dataTransfer.items;
    if (items.length === 1 && items[0].kind === "file") {
        e.stopPropagation();
        e.preventDefault();
        document.getElementById("drop-feedback").style.display = "block";
        document.getElementById("main").classList.add("blurred");
        e.dataTransfer.dropEffect = 'copy';
    }
}

function handleDragLeave(e) {
    e.stopPropagation();
    e.preventDefault();
    document.getElementById("drop-feedback").style.display = "none";
    document.getElementById("main").classList.remove("blurred");
}

function readTextFile(file) {
    return new Promise(function (resolve, reject) {
        var reader = new FileReader();
        reader.onload = function (e) {
            return resolve(e.target.result);
        };
        reader.readAsText(file);
        // TODO: detect error and reject promise
    });
}

// Polyfill for Safari's sake, which doesn't support
// `response.text()` or `response.arrayBuffer()`.
function fetchPolyFill(url, method, responseType, contentType, body, headers) {
    return new Promise(function (resolve, reject) {
        var xhr = new XMLHttpRequest();
        xhr.open(method, url, true);
        if (typeof responseType !== "undefined") {
            xhr.responseType = responseType;
        }
        if (typeof contentType !== "undefined") {
            xhr.setRequestHeader("Content-Type", contentType);
        }
        if (typeof headers !== "undefined") {
            for (var header in headers) {
                if (headers.hasOwnProperty(header)) {
                    xhr.setRequestHeader(header, headers[header]);
                }
            }
        }

        xhr.onreadystatechange = function (e) {
            if (this.readyState === XMLHttpRequest.DONE) {
                if (this.status >= 200 && this.status < 300) {
                    resolve(this);
                } else {
                    reject(this);
                }
            }
        };

        xhr.send(body || null);
    });
}

async function readMarkdownFile(file) {
    showResult();

    const markdown = await readTextFile(file);
    const wasm_module = await wasm;
    const latex = Converter.generateLatex(markdown, "markdown", wasm_module);

    document.querySelector('.result').classList.add('solo');
    document.getElementById("first-step-doclist").style.display = "none";

    var textarea = document.getElementById("latex");
    textarea.value = latex;
    setTimeout(function () { textarea.scrollTo(0, 0); }, 0);
}

function handleFileDrop(e) {
    handleDragLeave(e);

    const items = e.dataTransfer.items;
    if (items.length === 1 && items[0].kind === "file") {
        readMarkdownFile(e.dataTransfer.files[0]);
    }
}

function handleFileSelected(e) {
    readMarkdownFile(e.target.files[0]);
}

function copyLatex(e) {
    e.preventDefault();
    selectAllLatex();
    document.execCommand("copy");
    document.getElementById("confirm").innerText = "✓ Text copied to clipboard.";
    document.getElementById("confirm").style.visibility = "visible";
}

function saveZip(e) {
    // Don't prevent default.
    document.getElementById("confirm").innerText = '✓ ZIP file saved (check "Downloads" directory).';
    document.getElementById("confirm").style.visibility = "visible";
}

function selectAllLatex() {
    var textarea = document.getElementById("latex");
    textarea.select();
    textarea.setSelectionRange(0, 999999); /* For mobile devices */
    setTimeout(function () { textarea.scrollTo(0, 0); }, 0);
}

export function oauthCallback(token) {
    accessToken = token;
    getPaperDocs();
}

async function getPaperDocs() {
    document.getElementById("authorization-container").style.display = "none";
    document.getElementById("doc_list").style.display = "block";
    await Promise.all([getPaperDocsOldApi(), getPaperDocsNewApi()]);
    document.getElementById("wait-document").style.display = "none";
}

async function getPaperDocsOldApi() {
    const response = await getPaperDocsWithApi(
        "https://api.dropboxapi.com/2/paper/docs/list",
        { filter_by: "docs_accessed", sort_by: "modified", sort_order: "descending", limit: 100 }
    );
    return Promise.all(response.doc_ids.map((id, index) => downloadDocOldApi(id, index)));
}

async function getPaperDocsNewApi() {
    const response = await getPaperDocsWithApi(
        "https://api.dropboxapi.com/2/files/search",
        { path: "", query: ".paper", mode: { ".tag": "filename" } }
    );

    const list = response.matches.filter(m =>
        m.metadata.export_info
        && m.metadata.export_info.export_as === "html"
        && m.metadata.name.match(/^.*.paper$/)
    ).map(m => ({
        title: m.metadata.name.substr(0, m.metadata.name.length - 6),
        path: m.metadata.path_display,
        date: new Date(m.metadata.server_modified)
    })).sort((a, b) =>
        b.date - a.date
    );

    const parentNode = document.getElementById('doc_table');
    const nextSibling = parentNode.firstElementChild;

    for (var entry of list) {
        const el = makeListEntry(entry.title, null, entry.path, 0);
        parentNode.insertBefore(el, nextSibling);
    }
}

export async function getPaperDocsWithApi(endpoint, message) {
    const url = (
        endpoint + "?reject_cors_preflight=true&authorization=Bearer%20"
        + encodeURIComponent(accessToken)
    );
    const body = new TextEncoder("utf-8").encode(JSON.stringify(message)).buffer;

    try {
        var xhr;
        try {
            xhr = await fetchPolyFill(
                url, "POST", "text", "text/plain; charset=dropbox-cors-hack", body);
        } catch (e) {
            xhr = e;
        }
        var json = JSON.parse(xhr.response);
    } catch {
        document.getElementById("wait-document").style.display = "none";
        document.getElementById("error-document").style.display = "table-row";
        return;
    }

    if (json.error) {
        if (json.error['.tag'] === "invalid_access_token") {
            document.getElementById("doc_list").style.display = "none";
            document.getElementById("authorization-container").style.display = "block";
        } else {
            document.getElementById("wait-document").style.display = "none";
            document.getElementById("error-document").style.display = "table-row";
            document.getElementById("error-document-text").innerText = "An unknown error occurred."
        }
        throw null;
    } else {
        return json;
    }
}

var docs = {};

async function downloadDocOldApi(id, index) {
    var url = (
        'https://api.dropboxapi.com/2/paper/docs/download?reject_cors_preflight=true&authorization=Bearer%20'
        + encodeURIComponent(accessToken) + '&arg=' + encodeURIComponent(JSON.stringify({
            "doc_id": id,
            "export_format": "markdown"
        }))
    );

    try {
        var xhr = await fetchPolyFill(
            url, "POST", "text", "text/plain; charset=utf-8");
    } catch {
        console.log("Error while trying to download document meta data.");
    }

    const meta = JSON.parse(xhr.getResponseHeader("dropbox-api-result"));
    docs[id] = { meta, markdown: xhr.response };
    const entry = makeListEntry(meta.title, meta.owner, id, index);

    const parentNode = document.getElementById('doc_table');
    const refNode = Array.prototype.find.call(
        parentNode.childNodes,
        node => node.nodeName === 'TR' && node.getAttribute('data-index') > index
    );
    parentNode.insertBefore(entry, refNode);
}

async function downloadDocNewApi(path) {
    const url = "https://content.dropboxapi.com/2/files/export";
    const headers = {
        "Authorization": "Bearer " + encodeURIComponent(accessToken),
        "Dropbox-API-Arg": JSON.stringify({ path: path }),
    };
    var xhr = await fetchPolyFill(url, "POST", "text", undefined, null, headers);
    return xhr.response;
}

function makeListEntry(title, owner, id, index) {
    var tr = document.createElement('tr');
    tr.classList.add('document');
    tr.setAttribute('data-index', index);
    tr.setAttribute('data-docid', id);
    var td1 = document.createElement('td');
    td1.classList.add('list_icon');
    var img = document.createElement('img');
    img.src = 'document.svg';
    td1.appendChild(img);
    tr.appendChild(td1);

    var td2 = document.createElement('td');
    td2.classList.add('list_text');
    td2.appendChild(document.createTextNode(title));
    if (owner !== null) {
        var div = document.createElement('div');
        div.classList.add('doc_owner');
        div.appendChild(document.createTextNode(owner));
        td2.appendChild(div);
    }
    tr.appendChild(td2);

    tr.addEventListener("click", docSelected);
    return tr;
}

function showResult() {
    document.getElementById("latex").value = "Exporting Paper Document ..."
    document.getElementById("save-zip-container").style.display = "none";
    document.getElementById("wait-zip").style.display = "none";
    document.getElementById("choice-manual").style.display = "none";
    document.querySelector('#choice-oauth h2').style.display = "none";
    document.querySelector('#choice-oauth .steps').classList.add('active');
    document.getElementById("confirm").style.visibility = "hidden";
}

var docSelected, backToStart;
[docSelected, backToStart] = (function () {
    var selection = null;

    async function docSelected(e) {
        e.preventDefault();
        showResult();

        if (selection) {
            selection.classList.remove("selected");
        }
        selection = e.currentTarget;
        selection.classList.add("selected");

        const wasm_module = await wasm;
        const docId = selection.getAttribute("data-docid");
        var latex;
        if (docs[docId]) {
            // Old API.
            const markdown = await docs[docId].markdown;
            if (docId !== selection.getAttribute("data-docid")) {
                // Selection has changed while we were waiting for markdown or wasm to load.
                return;
            }
            latex = Converter.generateLatex(markdown, "markdown", wasm_module);
        } else {
            // New API.
            const html = await downloadDocNewApi(docId);
            if (docId !== selection.getAttribute("data-docid")) {
                // Selection has changed while we were waiting for html or wasm to load.
                return;
            }
            latex = Converter.generateLatex(html, "html", wasm_module);
        }

        var textarea = document.getElementById("latex");
        textarea.value = latex;
        setTimeout(function () { textarea.scrollTo(0, 0); }, 0);
    }

    function backToStart(e) {
        e.preventDefault();

        document.getElementById("choice-manual").style.display = "block";
        document.getElementById("first-step-doclist").style.display = "block";
        document.querySelector('#choice-oauth h2').style.display = "block";
        document.querySelector('#choice-oauth .steps').classList.remove('active');
        document.querySelector('.result').classList.remove('solo');

        if (selection) {
            selection.classList.remove("selected");
        }
    }

    return [docSelected, backToStart];
}());

const handleHashChange = (function () {
    var timeout = null;

    return function (e) {
        var target = document.getElementById(location.hash.substring(1));
        if (target.classList.contains('highlighter')) {
            var delay = 0;
            if (timeout !== null) {
                clearTimeout(timeout);
                timeout = null;
                target.classList.remove('highlight');
                delay = 200;
            }

            setTimeout(function () {
                target.classList.add('highlight');
                timeout = setTimeout(function () {
                    timeout = null;
                    target.classList.remove('highlight');
                }, 4100);
            }, delay);

            history.pushState(null, '', '#');
        }
    };
}());

document.addEventListener("DOMContentLoaded", onDomContentLoaded);
