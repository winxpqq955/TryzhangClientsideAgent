# ClientsideAgent

使用Forge TransformationService一个 bootstrap-native 用于启动子进程 agent.exe, 实现劫持modlauncher, mc与agent双向心跳, 修改进服IP(WIP)

agent.exe 主要功能是一个反向代理客户端, 首先会随机获取两个未被使用的端口 然后使用windows管道与bootstrap-native通讯 告诉bootstrap-native游戏转发的端口和与agent.exe的端口

玩家通过127.0.0.1:<random port> 连接到agent.exe反代转发服务器, 在agent.exe实现hwid, 反作弊等功能 (WIP)

agent使用私有协议(KCP, QUIC)与服务端通讯, 实现转发mc流量, 动态下发, 状态上报多种功能
