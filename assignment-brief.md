# COSC 473 – Decentralised Applications on the Web

## Assignment 2 - PeerBoard

### Overview

You will implement a PeerBoard node: a peer-to-peer distributed bulletin board where any two
correctly implemented nodes can discover each other, exchange messages, and maintain a
consistent local view of posted content. The protocol is shared across the entire class: a
correctly implemented node from one student should interoperate seamlessly with a correctly
implemented node from any other student.
PeerBoard has two layers:

- The bulletin board: peers publish and subscribe to topic-based message boards using
  GossipSub.
- The challenge system: peers can challenge each other to a game of Battleship using direct libp2p streams.
  The protocol layer is tightly specified (you must implement it correctly to earn marks). The user interface and any extensions are entirely your creative choice.

### The PeerBoard Protocol Specification

This section is the shared contract every implementation must satisfy. Think of it as an RFC you
are building to. Deviations, even small ones like a wrong protobuf field number, an incorrect
topic prefix, or a mismatched protocol ID string, will silently break interoperability. Read this
section carefully before writing any code.

1. Transport and Identity
   On startup, each node generates or loads a persistent Ed25519 keypair. The node’s PeerId is
   derived from this keypair. Save keypairs to disk (e.g. ~/.peerboard/identity.key) so the node
   retains its identity across restarts.(If you are running multiple nodes on the same machine you
   might want to make the file name a command line parameter).
   Transport must be TCP with Noise protocol encryption and Yamux multiplexing, in this order:
   TCP → noise (XX handshake) → yamux
   QUIC → (TLS 1.3 built-in, no yamux needed)
   The bootstrap node publishes two multiaddresses: one TCP and one QUIC. Hardcode both in
   your node. libp2p’s dialing logic will prefer QUIC and fall back to TCP automatically if QUIC is
   unreachable.
   When you need to represent your node’s PeerId as a string (e.g. in protobuf fields), always use
   peer_id.to_string() in Rust. This produces the canonical base58 btc-encoded multihash
   representation. Do not use any other encoding.
2. Peer Discovery – Kademlia DHT
   Nodes join the network via libp2p’s Kademlia DHT. You must configure Kademlia with the
   protocol ID /peerboard/kad/1.0.0. Do not use the libp2p default. The bootstrap node and all
   nodes developed for your projects must use this same protocol ID or they will not see each
   other in the DHT.
   // Required Kademlia configuration
   let mut kad_config = kad::Config::new(
   StreamProtocol::new("/peerboard/kad/1.0.0")
   );
   The bootstrap node’s multiaddresses (TCP and QUIC) will be published on Learn. On startup,
   nodes must:
   ● Dial all bootstrap peers from the published list.
   ● Run a self-lookup (find_node(self_peer_id)) to populate the routing table.
   The bootstrap node also acts as the rendezvous server (see section 5). Both services run on
   the same multiaddress: you dial it once and both behaviours are available over the same
   connection.
3. Bulletin Board – GossipSub
   All bulletin board communication uses GossipSub. Topic naming is strict:
    - Topics are UTF-8 strings with the prefix peerboard/v1/
    - The suffix is lowercase alphanumeric with hyphens only, matching the regex [a-z0-9-]+
    - Examples: peerboard/v1/general, peerboard/v1/rust, peerboard/v1/off-topic
      Message format
      Every message published to a GossipSub topic must be a valid PeerBoardMessage encoded in
      Protocol Buffers. The schema is fixed. You must use exactly this package name and field layout
      – any deviation produces an incompatible wire format:
      syntax = "proto3";
      package peerboard.v1;
      message PeerBoardMessage {
      string peer_id = 1; // peer_id.to_string() — base58btc multihash
      string topic = 2; // full topic, e.g. "peerboard/v1/rust"
      string content = 3; // post body; UTF-8 byte length ≤ 4096
      int64 timestamp = 4; // Unix timestamp in whole seconds
      string message_id = 5; // UUIDv4, e.g. "550e8400-e29b-41d4-a716-446655440000"
      string nickname = 6; // display name; UTF-8 byte length ≤ 32
      }
      Validation and rejection
      Nodes must silently drop any received message that fails any of the following checks. “Silently”
      means no error is logged to the user and no reply is sent to the publisher.
        - Protobuf decoding fails.
        - content.as_bytes().len() > 4096 – the limit is on UTF-8 encoded byte length, not
          character count.
        - topic does not begin with peerboard/v1/.
        - timestamp is more than 300 seconds (5 minutes) in the future. Use the system clock in
          UTC. A message timestamped in the past, even the distant past, is valid.
        - nickname.as_bytes().len() > 32 – same rule as content: byte length, not character
          count.
          Messages are deduplicated by message_id: if your node has already stored a message with a
          given message_id, silently drop any subsequent message with the same ID. Generate
          message_id values using UUIDv4 (random). The uuid crate with the v4 feature is pre-approved
          for this.
          Your node’s own published messages must pass all of these checks before being published.
          The interoperability test will deliberately send malformed messages and verify your node rejects
          them silently.
4. Local Message Store
   Your node must maintain a persistent local message store for all subscribed topics. SQLite via
   the rusqlite crate is recommended. The store must survive node restarts – messages
   received in a previous session must be present in the next. Deduplication by message_id must
   also be persistent: a message seen before a restart must not be stored again after one.
5. Challenge System – Battleship
   This is PeerBoard’s second major behaviour. Peers can challenge each other to a game of
   Battleship played directly between two nodes, not broadcast to a topic.
   How the challenge protocol uses libp2p
   The challenge system uses the request_response behaviour, the same behaviour you used
   in the tutorial example. Each shot is one request–response transaction: the shooter sends a
   Shot request, the opponent checks it against their private board and replies with a
   ShotResult. A new substream is opened for each transaction and closed when the
   response arrives. There is no persistent open stream for the lifetime of the game.
   Timeouts are handled automatically: configure the request_response behaviour with a
   30-second request timeout. If the opponent does not reply in time, you receive an
   OutboundFailure event – that is your timeout signal.
   Cheating detection is explicitly out of scope
   Each player maintains their own board privately and is trusted to report hit/miss outcomes
   honestly. You are not required to detect or prevent cheating. There is no mechanism in this
   protocol to verify that a player placed their ships correctly or reported outcomes truthfully.
   This is a deliberate simplification. Solving it properly requires cryptographic commitment
   schemes, which are beyond the scope of this assignment.
   Protocol IDs
   The challenge system uses two protocol IDs, each handled by its own request_response
   behaviour instance in your swarm:
   Protocol ID Request Response Purpose
   /peerboard/challenge/1.0.0 ChallengePropose ChallengeResponse Initiate or decline a
   game.
   /peerboard/battleship/1.0.0 BattleshipRequest BattleshipResponse All in-game
   exchanges.
   Using two separate behaviours keeps the handshake logic clean and isolated from the game
   logic. The /peerboard/battleship/1.0.0 behaviour is used for board-ready signalling, every
   shot, and the end-of-game notification — all wrapped in the envelope types below.
   Message schema
   All messages are protobuf-encoded. Use exactly this package name and field layout:
   syntax = "proto3";
   package peerboard.challenge.v1;
   // ── Handshake (protocol: /peerboard/challenge/1.0.0) ──────────────
   message ChallengePropose { string nickname = 1; }
   message ChallengeResponse { bool accepted = 1; }
   // ── In-game (protocol: /peerboard/battleship/1.0.0) ──────────────
   //
   // Every request and response is wrapped in an envelope so the
   // receiver always knows which message type has arrived.
   message BattleshipRequest {
   oneof msg {
   BoardReady board_ready = 1;
   Shot shot = 2;
   Resign resign = 3;
   }
   }
   message BattleshipResponse {
   oneof msg {
   BoardAck board_ack = 1;
   ShotResult shot_result = 2;
   ResignAck resign_ack = 3;
   }
   }
   // Sent by each player once ships are placed. Ship positions stay private.
   message BoardReady {}
   message BoardAck {}
   message Shot {
   uint32 seq = 1; // shot number, starting at 1
   uint32 col = 2; // 0–9, left to right
   uint32 row = 3; // 0–9, top to bottom
   }
   message ShotResult {
   uint32 seq = 1;
   bool hit = 2; // true if a ship occupies this cell
   bool sunk = 3; // true if this shot sank a ship entirely
   bool won = 4; // true if this was the last surviving ship cell
   }
   message Resign {}
   message ResignAck {}
   No message includes a from_peer_id field. The receiver already knows the sender’s PeerId
   from the authenticated connection.
   Rendezvous and matchmaking
   A node willing to play registers under the rendezvous namespace
   peerboard/challenge/seeking at the bootstrap node. A node looking for an opponent calls
   discover on the same namespace, picks a peer from the results, and sends them a
   ChallengePropose request. A node must unregister as soon as a game starts and re-register
   when it becomes available again.
   Board setup
   The board is a 10x10 grid. Each player places the following five ships privately before sending
   BoardReady:
   Ship Cells occupied
   Carrier 5
   Battleship 4
   Cruiser 3
   Submarine 3
   Destroyer 2
   Ships must be placed horizontally or vertically (not diagonally), must lie entirely within the grid,
   and must not overlap each other. Placement is never transmitted to the opponent and is never
   verified by the protocol. Each player is trusted to follow these rules.
   Session flow
   The challenger initiates. The full session proceeds as follows:
   Phase Requester Request Response Notes
   Handshake Challenger ChallengePropose ChallengeResponse If accepted = false, both
   sides stop. No further
   messages.
   Setup Challenger BoardReady BoardAck Challenger sends
   BoardReady once ships
   are placed and waits for
   BoardAck.
   Setup Opponent BoardReady BoardAck Opponent does the same
   independently. Game
   starts once both sides
   have completed this
   exchange. Challenger
   fires first.
   Play Challenger Shot (seq=1) ShotResult (seq=1) A coordinate on the
   opponent’s grid.
   Opponent replies with
   outcome.
   Play Opponent Shot (seq=2) ShotResult (seq=2) Now the opponent fires.
   The node that just
   received a ShotResult
   with won=false is always
   next to fire.
   … … … … Turns alternate until the
   game ends.
   End Loser Resign ResignAck Sent when a terminal
   condition is detected
   (see below).
   Turn ownership
   The rule is simple: the node that just received a ShotResult with won = false is responsible
   for sending the next Shot request. This means turn ownership transfers automatically with
   each ShotResult. Both nodes must enforce this locally. Do not send a Shot when it is not your
   turn.
   Shot validation
   On receiving a Shot, check it against your own board and reply with a ShotResult. You are the
   sole authority on your own board state. col and row must both be in the range 0–9. A shot
   outside this range is a protocol error – reply with a ShotResult where hit = false, sunk =
   false, won = false, then send a Resign request to end the game.
   A node must not fire the same coordinate twice. Track which cells you have already shot and do
   not repeat them.
   End game conditions
   A game ends when any of the following occurs. In all cases the ending node sends a Resign
   request (the request-response mechanism ensures the opponent receives it even under poor
   conditions) and stops sending further game messages:
   ● A ShotResult arrives with won = true. All of the opponent’s ships are sunk and you
   have won. Send Resign to signal the game is over. The opponent replies with
   ResignAck.
   ● An OutboundFailure event fires for a Shot request, because the opponent did not reply
   within the 30-second timeout. Send Resign to close out the game.
   ● The player wishes to concede at any point. Send Resign. The opponent replies with
   ResignAck and considers themselves the winner.
   On receiving a Resign request, always reply with ResignAck and consider the game over
   regardless of current state. Resign is always authoritative.
   Display, scoring across multiple games, and any logic beyond these end game conditions are
   your creative choice.
   Milestones and Suggested Plan
   Each milestone builds on the last. Do not skip ahead, a broken swarm loop will silently corrupt
   everything built on top of it.
   Step Goal How you know it works
   1 Project setup. Generate/load Ed25519
   keypair. TCP + Noise + Yamux and
   QUIC transport. Dial a bootstrap node.
   Node starts, prints its PeerId, connects to
   bootstrap without panicking.
   2 Kademlia DHT. Self-lookup. Routing
   table populated.
   Node logs at least 3 known peers after startup.
   3 GossipSub. Subscribe to
   peerboard/v1/general. Publish and
   receive PeerBoardMessages. Protobuf
   encode/decode. Validation/rejection.
   Node exchanges valid messages with a
   classmate’s node.
   4 SQLite (or other) message store.
   Rendezvous registration. Battleship:
   propose, accept/decline, board
   placement, shot/result exchange, end
   conditions.
   Full game completes between two nodes.
   Messages persist across restart.
   5 User interface. Polish. Interoperability
   testing against classmates and the
   reference node.
   Node passes all interoperability test cases.
   User Interface
   You must implement a user interface that allows a user to:
   ● Subscribe to and unsubscribe from topics.
   ● View messages received on subscribed topics.
   ● Post a new message to a topic.
   ● See a list of peers currently advertising themselves for a game.
   ● Send a challenge to a peer and play a game of Battleship to completion.
   Not all of these are required at every grade band — see the rubric below. The form the interface
   takes is entirely your choice:
   Approach Suggested crates Async complexity
   Command-line tool with
   subcommands
   clap Low – straightforward to wire to the swarm
   Terminal UI (TUI) with live
   message feed and game
   board
   ratatui, crossterm High – requires a channel bridge to the
   swarm event loop
   Local web server with
   browser front-end
   axum or warp (backend),
   any JS frontend
   High – same channel bridge requirement
   as TUI
   Any other approach that is
   functional and documented
   Your choice —
   Marking Rubric
   This assignment uses tiered grading. Each tier is a natural stopping point. You are not required
   to attempt the next tier to receive a good mark within your tier. The tiers build on each other: you
   cannot earn marks for a higher tier if a lower tier is incomplete or broken.
   Within each tier, UI quality and code quality act as modifiers that move your mark up or down
   within the band. Completing the full assignment with a polished, well-structured implementation
   and a thoughtful writeup earns A+..
   C band (60–69%) – Working bulletin board
   Your node connects to the network, participates in the Kademlia DHT, and exchanges valid
   bulletin board messages with other nodes.
   Component Requirement
   Transport + identity TCP and QUIC transports active. Ed25519 keypair generated and
   persisted. Node dials bootstrap successfully.
   Kademlia DHT Routing table populated after self-lookup. Auto-rebootstrap when
   table drops below 3 entries.
   GossipSub Publishes and receives valid PeerBoardMessages on correct topics.
   Protobuf encoding correct. Topic naming correct.
   Message validation Silently drops messages failing any validation rule. Own published
   messages pass all checks.
   UI – C band minimum Any interface that can post a message and display received
   messages. A minimal CLI is sufficient.
   Code quality + writeup Code compiles cleanly. README works on a fresh clone. Writeup
   addresses the two required prompts.
   B band (70–79%) – Persistent bulletin board
   Everything in the C band, plus your node persists messages across restarts and provides a
   complete, usable bulletin board interface.
   Component Requirement
   All C-band components Must be complete and passing interoperability checks.
   Message store All received messages stored persistently. Deduplication by
   message_id survives restarts. Messages from previous sessions
   present after restart.
   UI – B band Interface covers all bulletin board actions: subscribe, unsubscribe,
   post, view feed per topic.
   Code quality + writeup Writeup addresses all three prompts.
   A– to A+ band (80–100%) – Full assignment
   Everything in the B band, plus your node implements the full Battleship challenge system and
   passes all interoperability checks. The difference between A– and A+ is the quality of execution
   across protocol correctness, code structure, UI, and writeup depth.
   Component Requirement
   All B-band components Must be complete and passing interoperability checks.
   Rendezvous Node registers under peerboard/challenge/seeking when available.
   Correctly discovers peers. Unregisters when the game starts and
   re-registers when available again.
   Battleship protocol Full game over request–response: handshake, board placement,
   shot–result exchange, all terminal conditions. Both protocol IDs are
   correct. Passes all grader interoperability checks.
   UI – A band Interface covers all actions including challenge flow: browse available
   opponents, send/receive challenge, play a full game. More effort on
   this contributes to higher end of the band.
   Code quality + writeup Well-structured, idiomatic Rust throughout. Writeup is genuine and
   specific on all prompts. Exceptional writeup depth and code clarity
   contribute to A+.
   Libraries
   Required
   ● libp2p (latest stable) – must use the swarm and behaviour APIs directly; do not wrap
   with a higher-level abstraction that hides the behaviours.
   ● prost or prost-build – for protobuf encoding and decoding.
   ● tokio – async runtime.
   Other suggested crates
   ● rusqlite – local message persistence.
   ● ratatui, crossterm – terminal UI.
   ● axum, warp – web server for a browser-based UI.
   ● clap – CLI argument parsing.
   ● uuid – UUIDv4 generation for message IDs.
   ● tracing, tracing-subscriber – structured logging (recommended).
   Not permitted
   ● Any existing PeerBoard or bulletin board implementation. The point is to build it.
   ● Any crate that implements the PeerBoard or Battleship protocol on your behalf.
   Technical Writeup
   Submit a single page alongside your code. Address:
   ● One design decision: Describe one decision you made in your UI or data model that
   you are proud of. What alternatives did you consider and why did you choose this
   approach?
   ● One surprise: Describe one thing about libp2p’s Rust API that surprised you or took
   longer to understand than expected.
   ● Include a self-assessed grade estimation with an explanation based on the grading
   criteria.
   The writeup is a reflection exercise, not a report. It is graded on whether it is genuine and
   specific, a page of vague generalities will not earn full marks.
   Submission Requirements
   ● Submit your code as a repository on eng-git and add me (bta47) to the project (as a
   reporter user or higher access).
   ● Include a README.md file with clear, step-by-step instructions on how to build and run
   your application.
   ● Upload writeup to dropbox on Learn
