cmd_Release/x11.node := ln -f "Release/obj.target/x11.node" "Release/x11.node" 2>/dev/null || (rm -rf "Release/x11.node" && cp -af "Release/obj.target/x11.node" "Release/x11.node")
