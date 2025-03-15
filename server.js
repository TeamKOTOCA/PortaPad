const express = require('express');
const path = require('path');
const http = require('http');//httpsサーバー
const WebSocket = require('ws');
const robot = require('robotjs');

const app = express();
const server = http.createServer(app); // HTTPサーバー作成
const ws = new WebSocket.Server({ server }); // WebSocketをHTTPサーバーに統合

// /静的ファイルを公開 (http://localhost:3000/)
app.use('/', express.static(path.join(__dirname, 'html')));

// WebSocketの接続処理
ws.on('connection', (ws, req) => {
    console.log('WebSocketの接続が行われました');
    var righttouch = false;

        ws.on('close', () => {

            console.log('WebSocketのせつぞくがきれました');
        });

        ws.on('message', (message) => {
            const messageString = message.toString();
            let massages = messageString.split(',');
            console.log(massages[0]);
            if(massages[0] == "lefclick"){
                robot.mouseClick();
                console.log("clicked");
                righttouch = false;
            }else if(massages[0] == "rigclick"){
                if(righttouch == false){
                    robot.mouseClick('right');
                    console.log("Rclicked");
                }
                righttouch = true;
            }else if(massages[0] == "cursol"){
                const mousePos = robot.getMousePos();
                const x = mousePos.x + Number(massages[1])*3;
                const y = mousePos.y + Number(massages[2])*3;
                robot.moveMouse(x,y);
                console.log(x + "," + y);
            }else if(massages[0] == "scroll"){
                const mousePos = robot.getMousePos();
                const x = Number(massages[1]);
                const y = Number(massages[2]);
                robot.scrollMouse(x,y);
                console.log(x + "," + y);
            }
        })

        // 30秒ごとにPingを送る
        const pingInterval = setInterval(() => {
            ws.ping();
        }, 30000);
        // クライアントからPongを受信したとき
        ws.on('pong', () => {});
});


//サーバーを起動
const PORT = 3000;
server.listen(PORT, () => {
    console.log(`Server is started at http://localhost:${PORT}`);
});


//console上での終了操作
const readline = require('readline');

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});
console.log('終了はqまたはｑと入力');

rl.on('line', (input) => {
    if (input.trim() === 'q' || input.trim() === "ｑ") {
        console.log('終了します。');
        rl.close();
        process.exit(0);
    }
});