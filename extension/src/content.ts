import browser from "webextension-polyfill";
import type { Msg } from "./backgroundPage";

declare global {
    interface Window {
        hasRun: boolean;
        terms: string | null;
    }
}

function main() {
    if (window.hasRun) {
        return;
    }
    window.hasRun = true;

    const input = document.querySelectorAll(
        'input[title="Search"], input[aria-label="Search"]',
    );
    const searchBar = <HTMLInputElement>input.item(0);
    if (searchBar == null) {
        // This is a page without a search bar. Ignore it.
        return;
    }

    const content = searchBar.value;
    let message: Msg = { type: "NewSearch", content: content };
    browser.runtime.sendMessage(message).catch((err) => {
        console.log(err);
    });

    searchBar.addEventListener(
        "change",
        function (this: HTMLInputElement, _event) {
            const content = this.value;
            console.log(content);
            let message: Msg = { type: "NewSearch", content: content };
            browser.runtime.sendMessage(message).catch((err) => {
                console.log(err);
            });
        },
    );
}

console.log("Loaded!");
main();
