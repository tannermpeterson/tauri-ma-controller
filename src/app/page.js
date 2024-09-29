"use client";
import { invoke } from "@tauri-apps/api/tauri";
import { listen } from "@tauri-apps/api/event";
import { useState } from "react";

listen("number", (event) => {
  console.log(`received ${event.payload}`);
});

// TODO capitalize log msgs
const handleConnectBtnPress = (connected, cb) => {
  if (connected) {
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

const handleStreamBtnPress = (streaming) => {
  let command;
  if (streaming) {
    console.log("stopping data stream");
    command = "stop_data_stream";
    invoke(command)
      .then(() => console.log(`${command} command processed`))
      .catch((error) => console.error(`${command} command errored:`, error));
  } else {
    console.log("starting data stream");
    command = "start_data_stream";
  }
  invoke(command)
    .then(() => console.log(`${command} command processed`))
    .catch((error) => console.error(`${command} command errored:`, error));
};

const handleRecordBtnPress = (recording, cb) => {
  if (recording) {
    console.log("stopping recording");
    const command = "stop_recording";
    invoke(command, { commandInfo: {} })
      .then(() => console.log(`${command} command processed`))
      .catch((error) => console.error(`${command} command errored:`, error));
  } else {
    console.log("starting recording");
    const command = "start_recording";
    invoke(command, {
      commandInfo: { plate_barcode: null, stim_barcode: null, is_calibration_recording: false },
    })
      .then(() => console.log(`${command} command processed`))
      .catch((error) => {
        console.error(`${command} command errored:`, error);
        cb();
      });
  }
};

export default function Home() {
  const [connected, setConnected] = useState(false);
  const [streaming, setStreaming] = useState(false);
  const [recording, setRecording] = useState(false);

  const connectButtonText = connected ? "disconnect" : "connect";
  const streamButtonText = streaming ? "stop stream" : "start stream";
  const recordButtonText = recording ? "stop recording" : "start recording";

  return (
    <>
      <button
        onClick={() => {
          handleConnectBtnPress(connected, () => {
            setConnected(false);
            setStreaming(false);
            setRecording(false);
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
      <button
        onClick={() => {
          handleRecordBtnPress(recording, () => {
            setRecording(false);
          });
          setRecording(!recording);
        }}
        disabled={!streaming}
      >
        {recordButtonText}
      </button>
    </>
  );
}
