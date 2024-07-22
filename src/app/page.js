"use client";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useState } from "react";

listen("number", (event) => {
  console.log(`received ${event.payload}`);
});

const handleConnectBtnPress = (running, cb) => {
  if (running) {
    console.log("disconnecting");
    invoke("cancel");
  } else {
    console.log("connecting");
    const command = "connect";
    invoke(command)
      .then((message) => {
        console.log(`${command} command processed:`, message);
        cb();
      })
      .catch((error) => {
        console.error(`${command} command errored:`, error);
        cb();
      });
  }
};

const handleStreamBtnPress = (running) => {
  if (running) {
    console.log("stopping data stream");
    const command = "stop_data_stream";
    invoke(command)
      .then(() => console.log(`${command} command processed`))
      .catch((error) => console.error(`${command} command errored:`, error));
  } else {
    console.log("starting data stream");
    const command = "start_data_stream";
    invoke(command)
      .then(() => console.log(`${command} command processed`))
      .catch((error) => console.error(`${command} command errored:`, error));
  }
};

export default function Home() {
  const [connected, setConnected] = useState(false);
  const [streaming, setStreaming] = useState(false);

  const connectButtonText = connected ? "disconnect" : "connect";
  const streamButtonText = streaming ? "stop stream" : "start stream";

  return (
    <>
      <button
        onClick={() => {
          handleConnectBtnPress(connected, () => {
            setConnected(false);
            setStreaming(false);
          });
          setConnected(!connected);
        }}
      >
        {connectButtonText}
      </button>
      <button
        onClick={() => {
          handleStreamBtnPress(streaming);
          setStreaming(!streaming);
        }}
        disabled={!connected}
      >
        {streamButtonText}
      </button>
    </>
  );
}
