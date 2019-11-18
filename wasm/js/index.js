const wasm = import("../pkg/index.js");

const Converter = (function () {
    var queue = {};
    var fileNames = [];
    var generation = 0;

    function clearQueue() {
        queue = {};
        fileNames = [];
        generation += 1;
    }

    var addToQueue = function (wasm_module) {
        return function (url) {
            if (!queue.hasOwnProperty(url)) {
                const match = url.match(/_([^_]+?)\.(png|jpg|jpeg)$/i);
                if (match) {
                    var suffix = "." + match[2].toLowerCase();
                    var fileName = match[1] + suffix;
                    var i = 1;
                    while (fileNames.indexOf(fileName) >= 0) {
                        i += 1;
                        fileName = match[1] + "-" + i + suffix;
                    }
                    fileNames.push(fileName);

                    queue[url] = fetchPolyFill(url, "arraybuffer").then(buf => {
                        wasm_module.register_image(url, fileName, new Uint8Array(buf));
                    });
                }
            }
        };
    };

    function markdownToLatex(markdown, wasm_module) {
        clearQueue();
        document.getElementById("save-zip-container").style.display = "none";
        const latex = wasm_module.markdown_to_latex(markdown, addToQueue(wasm_module));
        if (fileNames.length !== 0) {
            document.getElementById("wait-zip").style.display = "inline";
            const originalGeneration = generation;
            Promise.all(Object.values(queue)).then(function () {
                if (generation === originalGeneration) {
                    const zipFileData = wasm_module.markdown_to_zipped_latex(markdown);
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

    function generateZipFile(wasm_module) {
        return wasm_module.markdown_to_zipped_latex(markdown);
    }

    return {
        markdownToLatex,
        generateZipFile,
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
        getPaperDocs(savedTokens.tokens[savedTokens.last_used]);
    }
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

// Polyfill for Safari, which does not support `response.text()` or `response.arrayBuffer()`.
// `responseType` can be "text" or "arraybuffer";
function fetchPolyFill(url, responseType) {
    return new Promise(function (resolve, reject) {
        var xhr = new XMLHttpRequest();
        xhr.open('GET', url, true);
        xhr.responseType = 'arraybuffer';

        xhr.onload = function (e) {
            if (this.status >= 200 && this.status < 300) {
                resolve(this.response);
            }
        };

        xhr.send();
    });
}

async function readMarkdownFile(file) {
    const markdown = await readTextFile(file);
    const wasm_module = await wasm;
    const latex = Converter.markdownToLatex(markdown, wasm_module);

    showResult();
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

export async function getPaperDocs(accessToken) {
    document.getElementById("authorization-container").style.display = "none";
    document.getElementById("doc_list").style.display = "block";

    const url = (
        'https://api.dropboxapi.com/2/paper/docs/list?reject_cors_preflight=true&authorization=Bearer%20'
        + encodeURIComponent(accessToken)
    );
    const message = {
        filter_by: "docs_accessed",
        sort_by: "modified",
        sort_order: "descending",
        limit: 100
    };

    try {
        // TODO: does not work on Safari.
        var response = await fetch(url, {
            method: 'POST',
            mode: 'cors',
            cache: 'no-cache',
            headers: {
                'Content-Type': 'text/plain; charset=dropbox-cors-hack',
            },
            body: JSON.stringify(message)
        });
        var json = JSON.parse(await response.text());
    } catch {
        document.getElementById("wait-document").style.display = "none";
        document.getElementById("error-document").style.display = "table-row";
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
    } else {
        await Promise.all(json.doc_ids.map((id, index) => downloadDoc(accessToken, id, index)));
        document.getElementById("wait-document").style.display = "none";
    }
}

var docs = {};

async function downloadDoc(accessToken, id, index) {
    var url = (
        'https://api.dropboxapi.com/2/paper/docs/download?reject_cors_preflight=true&authorization=Bearer%20'
        + encodeURIComponent(accessToken) + '&arg=' + encodeURIComponent(JSON.stringify({
            "doc_id": id,
            "export_format": "markdown"
        }))
    );

    try {
        // TODO: does not work on Safari.
        var response = await fetch(url, {
            method: 'POST',
            mode: 'cors',
            cache: 'no-cache',
            headers: {
                'Content-Type': 'text/plain; charset=utf-8',
            },
        });
    } catch {
        console.log("Error while trying to download document meta data.");
    }

    const meta = JSON.parse(response.headers.get("dropbox-api-result"));
    docs[id] = { meta, markdown: response.text() };

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
    td2.appendChild(document.createTextNode(meta.title));
    var div = document.createElement('div');
    div.classList.add('doc_owner');
    div.appendChild(document.createTextNode(meta.owner));
    td2.appendChild(div);
    tr.appendChild(td2);

    tr.addEventListener("click", docSelected);

    const parentNode = document.getElementById('doc_table');
    const refNode = Array.prototype.find.call(
        parentNode.childNodes,
        node => node.nodeName === 'TR' && node.getAttribute('data-index') > index
    );
    parentNode.insertBefore(tr, refNode);
}

function showResult() {
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

        const docId = selection.getAttribute("data-docid");
        const markdown = await docs[docId].markdown;
        const wasm_module = await wasm;

        if (docId !== selection.getAttribute("data-docid")) {
            // Selection has changed while we were waiting for markdown or wasm to load.
            return;
        }

        const latex = Converter.markdownToLatex(markdown, wasm_module);
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
