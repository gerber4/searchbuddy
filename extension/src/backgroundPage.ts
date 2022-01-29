import browser, { Runtime } from "webextension-polyfill";
import MessageSender = Runtime.MessageSender;

export type Msg = { type: "NewSearch"; content: string };

let terms: string | null = null;

browser.runtime.onMessage.addListener((message: Msg, sender: MessageSender) => {
    switch (message.type) {
        case "NewSearch":
            terms = message.content;

            if (terms === null || terms === "") {
                browser.browserAction
                    .setIcon({ path: "sleep-128.png" })
                    .catch((err) => {
                        console.log(err);
                    });
            } else {
                browser.browserAction
                    .setIcon({ path: "starry-128.png" })
                    .catch((err) => {
                        console.log(err);
                    });
            }

            console.log(message.content);
    }
});

browser.browserAction.onClicked.addListener(() => {
    if (terms !== null && terms !== "") {
        browser.tabs
            .create({
                url: `searchbuddy.html?terms=${terms}`,
            })
            .catch((err) => {
                console.log(err);
            });
    }
});
