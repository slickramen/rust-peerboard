# rust-peerboard

<img src="./image.png" alt="drawing" width="500"/>

A peer-to-peer distributed bulletin board.

## About

This was created for a Univeristy Assignment, COSC473 - Decentralized Web Applications. The backend server is implemented in Rust and the frontend is a React web app.

## How to install

After cloning the repository,

### Backend

From the repository root, run

```
cargo build
```

This will compile the backend server.

### Frontend

From the repository root, run

```
cd frontend

npm install
```

This will install the required frontend dependencies

## How to run

### Backend

To run the backend, from the repository root, run

```
cargo run
```

A nickname can optionally be passed in using the `-n` argument, e.g.:

```
cargo run -- -n "student"
```

This will initialise your nickname as "student". If no nickname is passed in, it will default to "anon".

### Frontend

To run the frontend, from the repository root, run

```
cd frontend

npm run dev
```

You must ensure that the backend is runnning, otherwise you won't be able to interact with the application.

## Functionality

This peerboard implements basic peer-to-peer chat functionality, such as subscribing and unsubscribing to/from different topics and messaging. Messages are stored locally in a SQLite database.

The icons used for user profiles were generated using a tool I created, which you can find [here](https://github.com/slickramen/sl-identicon).
