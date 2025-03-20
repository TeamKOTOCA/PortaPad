// Denoの必要なモジュールをインポート
import { serveDir } from "https://deno.land/std@0.224.0/http/file_server.ts";
import { WebSocketServer } from "https://deno.land/x/websocket@v0.1.4/mod.ts";
import * as autoOSDeno from "../autoOSDeno/mouse.ts";

// WebSocketサーバーを作成
const wss = new WebSocketServer(3001);

// "apps"ディレクトリから静的ファイルを提供する
async function handleHttpRequest(req: Request){
  const _url = new URL(req.url);
  
  // "apps"ディレクトリから静的ファイルを提供
  return await serveDir(req, {
    fsRoot: "html",
    urlRoot: "",
  });
}

// HTTPサーバーを起動
const httpServer = Deno.serve({ port: 3000 }, handleHttpRequest);
console.log(`サーバーが起動しました: http://localhost:3000`);

// WebSocket接続を処理
wss.on("connection", (ws) => {
  console.log("WebSocketの接続が行われました");
  let righttouch = false;

  ws.on("close", () => {
    console.log("WebSocketのせつぞくがきれました");
  });

  ws.on("message", (message) => {
    try{
      const messageString = typeof message === "string" ? message : new TextDecoder().decode(message);
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
          const x = mousePos[0] + Number(massages[1]) * 3;
          const y = mousePos[1] + Number(massages[2]) * 3;
          console.log(x + "," + y);
          autoOSDeno.MoveMouse(x, y);
        }
      } else if (massages[0] == "scroll") {
        if(massages[1] == "up"){
          autoOSDeno.ClickMouse(4);
        }else{
          autoOSDeno.ClickMouse(5);
        }
      } else if (massages[0] == "drag") {
        autoOSDeno.ChangeMouse(1,1);
        let mousePos: number[]| undefined  = [0, 0];
        mousePos = autoOSDeno.GetMouse();
        console.log(mousePos);
        if (mousePos != null && massages.length >= 1) {
          const x = mousePos[0] + Number(massages[1]) * 3;
          const y = mousePos[1] + Number(massages[2]) * 3;
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
  });

});

// コンソール入力でのシャットダウン処理
const textDecoder = new TextDecoder();
console.log("終了はqまたはｑと入力");

// 標準入力から読み取り
const readLines = async () => {
  for await (const line of Deno.stdin.readable) {
    const input = textDecoder.decode(line).trim();
    if (input === "q" || input === "ｑ") {
      console.log("終了します。");
      httpServer.shutdown();
      Deno.exit(0);
    }
  }
};

readLines();
