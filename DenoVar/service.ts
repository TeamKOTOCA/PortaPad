// Denoの必要なモジュールをインポート
import * as autoOSDeno from "../../autoOSDeno/mouse.ts";
import { serveTls } from "https://deno.land/std@0.156.0/http/server.ts";

let righttouch = false;
let scrolled = 0;

const certFile = "./public.crt"; // 証明書のパス
const keyFile = "./private.key"; // 秘密鍵のパス
const port = 8000; // ポート番号
serveTls((req) => {
  // `Deno.upgradeWebSocket(req)`でHTTP接続をWebSocket用に切り替え
  const { response, socket } = Deno.upgradeWebSocket(req);

      // 接続を開いた時の処理
      socket.addEventListener("open", () => console.log("open!"));
      // メッセージを受信した時の処理
      socket.addEventListener("message", (message) => getmessage(message));
      // エラー発生時の処理
      socket.addEventListener("error", () => console.log("error!"));
      // 接続を閉じた時の処理
      socket.addEventListener("close", () => console.log("close!"));

  // レスポンスを返してWebSocket通信スタート
  return response;
}, { certFile, keyFile, port });


function getmessage(message: MessageEvent) {
  try{
    const messageString = message.data;
    const massages = messageString.split(",");
    console.log(massages[0]);
    
    if (massages[0] == "lefclick") {
      autoOSDeno.ClickMouse(1);
      console.log("clicked");
      righttouch = false;
    } else if (massages[0] == "rigclick") {
      if (righttouch == false) {
        autoOSDeno.ClickMouse(3);
        console.log("Rclicked");
      }
      righttouch = true;
    } else if (massages[0] == "cursol") {
      let mousePos: number[]| undefined  = [0, 0];
        mousePos = autoOSDeno.GetMouse();
        console.log(mousePos);
      if (mousePos != null && massages.length >= 1) {
        const x = mousePos[0] + Number(massages[1]) * 4;
        const y = mousePos[1] + Number(massages[2]) * 4;
        console.log(x + "," + y);
        autoOSDeno.MoveMouse(x, y);
      }
    } else if (massages[0] == "scroll") {
      if (scrolled >= 3) {
        if(massages[1] == "up"){
          autoOSDeno.ClickMouse(4);
        }else{
          autoOSDeno.ClickMouse(5);
        }
        scrolled = 0;
      }
      scrolled++;
    } else if (massages[0] == "drag") {
      autoOSDeno.ChangeMouse(1,1);
      let mousePos: number[]| undefined  = [0, 0];
      mousePos = autoOSDeno.GetMouse();
      console.log(mousePos);
      if (mousePos != null && massages.length >= 1) {
        const x = mousePos[0] + Number(massages[1]) * 4;
        const y = mousePos[1] + Number(massages[2]) * 4;
        console.log(x + "," + y);
        autoOSDeno.MoveMouse(x, y);
      }
    } else if (massages[0] == "end") {
      autoOSDeno.ChangeMouse(1,0);
      autoOSDeno.ChangeMouse(3,0);
    }
  }catch(e){
    console.error(e);
  }
}

// コンソール入力でのシャットダウン処理
const textDecoder = new TextDecoder();
console.log("終了はqまたはｑと入力");

// 標準入力から読み取り
const readLines = async () => {
  for await (const line of Deno.stdin.readable) {
    const input = textDecoder.decode(line).trim();
    if (input === "q" || input === "ｑ") {
      console.log("終了します。");
      Deno.exit(0);
    }
  }
};

readLines();
