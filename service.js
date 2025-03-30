const inputter = require("./modules/build/Release/x11");
const readline = require('readline');

let righttouch = false;
let scrolled = 0;

function getmessage(message) {
  try{
    const messageString = message.data;
    const massages = messageString.split(",");
    console.log(massages[0]);
    
    if (massages[0] == "lefclick") {
      inputter.ClickMouse(1);
      console.log("clicked");
      righttouch = false;
    } else if (massages[0] == "rigclick") {
      if (righttouch == false) {
        inputter.ClickMouse(3);
        console.log("Rclicked");
      }
      righttouch = true;
    } else if (massages[0] == "cursol") {
      let mousePos = [0, 0];
        mousePos = inputter.GetMouse();
        console.log(mousePos);
      if (mousePos != null && massages.length >= 1) {
        const x = mousePos[0] + Number(massages[1]) * 4;
        const y = mousePos[1] + Number(massages[2]) * 4;
        console.log(x + "," + y);
        inputter.MoveMouse(x, y);
      }
    } else if (massages[0] == "scroll") {
      if (scrolled >= 3) {
        if(massages[1] == "up"){
          inputter.ClickMouse(4);
        }else{
          inputter.ClickMouse(5);
        }
        scrolled = 0;
      }
      scrolled++;
    } else if (massages[0] == "drag") {
      inputter.ChangeMouse(1,1);
      let mousePos = [0, 0];
      mousePos = inputter.GetMouse();
      console.log(mousePos);
      if (mousePos != null && massages.length >= 1) {
        const x = mousePos[0] + Number(massages[1]) * 4;
        const y = mousePos[1] + Number(massages[2]) * 4;
        console.log(x + "," + y);
        inputter.MoveMouse(x, y);
      }
    } else if (massages[0] == "end") {
      inputter.ChangeMouse(1,0);
      inputter.ChangeMouse(3,0);
    }
  }catch(e){
    console.error(e);
  }
}

//console上での終了操作

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
