import browser, { Runtime } from "webextension-polyfill";
import MessageSender = Runtime.MessageSender;

export type Msg = { type: "NewSearch"; content: string };

let activeTab: number | null = null;
let terms: Record<number, string | null> = {};

function updateIcon() {
    if (
        activeTab != null &&
        terms[activeTab] != null &&
        terms[activeTab] != ""
    ) {
        browser.browserAction
            .setIcon({ path: "exclaim-128.png" })
            .catch((err) => {
                console.log(err);
            });
    } else {
        browser.browserAction
            .setIcon({ path: "sleep-128.png" })
            .catch((err) => {
                console.log(err);
            });
    }
}

browser.tabs.onActivated.addListener((activeInfo) => {
    activeTab = activeInfo.tabId;
    updateIcon();
});

browser.runtime.onMessage.addListener((message: Msg, sender: MessageSender) => {
    switch (message.type) {
        case "NewSearch":
            if (sender.tab?.id != undefined) {
                terms[sender.tab.id] = message.content;
            }

            updateIcon();

            console.log(message.content);
    }
});

browser.browserAction.onClicked.addListener(() => {
    if (
        activeTab != null &&
        terms[activeTab] != null &&
        terms[activeTab] != ""
    ) {
        browser.tabs
            .create({
                url: `searchbuddy.html?terms=${terms[activeTab]}`,
            })
            .catch((err) => {
                console.log(err);
            });
        updateIcon();
    }
});
