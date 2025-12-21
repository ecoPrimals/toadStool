# 🎮 Multi-Tower LAN Gaming Setup

**Test the complete ecoPrimals gaming network!**

---

## 🗼 Scenario: Two Towers, One Game

### The Plan

1. **Tower 1** (this machine): Run dedicated server
2. **Tower 2** (your other machine): Pull code and join
3. **Demonstrate**: Seamless LAN gaming across ecoPrimals infrastructure!

---

## 🚀 Quick Start

### On Tower 1 (Server Host)

```bash
cd /home/eastgate/Development/ecoPrimals/toadstool/showcase/gaming-evolution

# Start dedicated server
./start_dedicated_server.sh

# Server will show:
# - IP address
# - Port (27960)
# - How others can connect
```

**Write down the IP address shown!** (e.g., `192.168.1.100`)

### On Tower 2 (Client)

```bash
# Clone/pull the repo
cd /home/eastgate/Development/ecoPrimals/toadstool
git pull

# Navigate to showcase
cd showcase/gaming-evolution

# Join the server
./join_lan_server.sh 192.168.1.100

# Or use the in-game menu!
```

---

## 📋 Detailed Instructions

### Tower 1: Server Setup

1. **Start the server**:
   ```bash
   ./start_dedicated_server.sh
   ```

2. **Note the output**:
   ```
   Server IP: 192.168.1.100
   Port: 27960
   ```

3. **Server stays running** - players can join/leave anytime!

4. **To stop**: Press `Ctrl+C`

### Tower 2: Join Game

**Option 1: Auto-connect script**
```bash
./join_lan_server.sh 192.168.1.100
```

**Option 2: In-game menu**
```bash
openarena
# Then: Multiplayer → Specify Server → Enter IP
```

**Option 3: Console**
```bash
openarena
# Press ~ (tilde) for console
# Type: /connect 192.168.1.100
```

**Option 4: LAN Discovery**
```bash
openarena
# Multiplayer → Local Servers
# Wait for server to appear
# Double-click to join!
```

---

## 🎯 Popular Maps for Testing

### Fast Action (2-4 players)
```bash
./start_dedicated_server.sh dm6
```
- Small arena
- Intense combat
- Quick matches

### Medium Action (4-8 players)
```bash
./start_dedicated_server.sh dm7
```
- Balanced map
- Good weapon variety
- Classic gameplay

### Epic Action (8+ players)
```bash
./start_dedicated_server.sh dm17
```
- Space platforms
- Long range combat
- Spectacular!

---

## 🔧 Troubleshooting

### "Waiting for challenge..."

This means client can't reach server. Check:

1. **Firewall**: Allow port 27960
   ```bash
   sudo ufw allow 27960/udp
   sudo ufw allow 27960/tcp
   ```

2. **Same network**: Both machines on same LAN
   ```bash
   # Check connectivity
   ping 192.168.1.100
   ```

3. **Correct IP**: Use the IP shown by server
   ```bash
   hostname -I
   ```

### Server not visible in LAN list

1. **Wait 30 seconds** - discovery takes time
2. **Refresh**: Press refresh in-game
3. **Direct connect**: Use "Specify Server" instead

### Connection refused

1. **Check server is running**:
   ```bash
   # On server machine
   netstat -an | grep 27960
   ```

2. **Check port forwarding** (if on different subnets)

3. **Restart server**:
   ```bash
   # Stop with Ctrl+C
   ./start_dedicated_server.sh
   ```

---

## 🌐 Network Topology

### Simple LAN (Most Common)

```
Router
  ├── Tower 1 (192.168.1.100) - Server
  └── Tower 2 (192.168.1.101) - Client
```

Both on same network = works perfectly!

### Different Subnets (Advanced)

```
Tower 1 (10.0.0.100) ←→ VPN ←→ Tower 2 (192.168.1.100)
```

Need:
- VPN tunnel (WireGuard/Beardog)
- Or Songbird federation
- Port forwarding

---

## 🎊 Testing Checklist

### Tower 1 (Server)

- [ ] Code pulled/updated
- [ ] Server script executable
- [ ] Server launched successfully
- [ ] IP address noted
- [ ] Firewall allows port 27960
- [ ] Server accessible (netstat check)

### Tower 2 (Client)

- [ ] Code pulled/updated
- [ ] Can ping server IP
- [ ] Join script executable
- [ ] OpenArena installed
- [ ] Connected successfully
- [ ] Can see/shoot other players!

---

## 🎮 Demo Workflow

### Perfect Demo Sequence

1. **Start server on Tower 1**
   ```bash
   ./start_dedicated_server.sh dm17
   ```

2. **Show server info**
   - IP address
   - Port
   - "Server is now waiting for players!"

3. **Switch to Tower 2**
   ```bash
   git pull
   cd showcase/gaming-evolution
   ./join_lan_server.sh 192.168.1.100
   ```

4. **Play together!**
   - Show seamless join/leave
   - Demonstrate bots filling slots
   - Show map changes work
   - Leave and rejoin

5. **Add 3rd player** (optional)
   - Friend's laptop
   - Another tower
   - Show scaling!

---

## 💡 Advanced: Songbird Integration

### Future Enhancement

Once Songbird federation APIs are implemented:

```bash
# Tower 1: Advertise game server
curl -X POST http://localhost:8080/api/gaming/advertise \
  -d '{"game": "openarena", "port": 27960}'

# Tower 2: Discover game servers
curl http://localhost:8080/api/gaming/discover

# Auto-join via Songbird!
```

This showcases the **full ecoPrimals vision**! 🚀

---

## 📊 What This Demonstrates

### ecoPrimals Capabilities

✅ **Distributed Gaming** - Games across multiple towers  
✅ **Zero Configuration** - Just run scripts!  
✅ **Open Source** - All free software  
✅ **Self-Hosted** - Your hardware, your network  
✅ **Scalable** - Add more towers anytime  
✅ **Real-World** - Actual gameplay, not mock  

### vs Traditional Platforms

| Feature | Traditional | ecoPrimals |
|---------|-------------|------------|
| **Setup** | Complex | One script |
| **Cost** | Subscription | Free |
| **Control** | Limited | Complete |
| **Privacy** | Cloud | LAN only |
| **Latency** | Internet | LAN speed |

---

## 🎉 Success!

When you see:
- ✅ Server running on Tower 1
- ✅ Client connected from Tower 2
- ✅ Both players in same game
- ✅ Can shoot each other!
- ✅ Can leave/rejoin anytime

**You've proven the ecoPrimals gaming platform works!** 🎊🚀

---

## 🚀 Next Steps

After proving multi-tower gaming:

1. **Test with more games** (0 A.D., SuperTuxKart)
2. **Add Songbird discovery**
3. **Implement federation APIs**
4. **Add Steam library access**
5. **Create video demo!**

---

## 📝 Notes

**Key IPs for your setup**:
- Tower 1: `_____________`
- Tower 2: `_____________`
- Tower 3: `_____________`

**Firewall commands used**:
```bash
sudo ufw allow 27960
```

**Successfully tested maps**:
- [ ] dm17 (The Longest Yard)
- [ ] dm6 (The Campgrounds)
- [ ] dm7 (Abandoned Base)

---

**Ready to test multi-tower gaming!** 🎮✨

