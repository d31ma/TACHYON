# Answer the questions in a terminal:
ty init my-app

# Or give every answer as an option:
ty init my-app --realm both --tac ts --yon py,go --target web,ios --features sockets,http-client

# Install every missing tool without asking:
ty init my-app --tac rs --yes

# Grow the app later:
ty add language server java
ty add target android
ty add feature topics
