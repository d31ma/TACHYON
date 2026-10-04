ty init my-app --name "My App"
ty init my-app --realm both --tac ts --yon py --target web,ios --features sockets --yes
ty add language server go
ty add target android
ty add feature topics
ty preview
ty preview --host shop.localhost
ty bundle
ty bundle --target macos,ios
ty bundle --native --target macos
ty start
ty doctor
ty test
ty test --coverage
ty types
ty api freeze
ty handler invoke server/controllers/ItemsController.py --route /items --method GET
ty migrate check
ty migrate fix
ty migrate fix --write
ty cache status
ty cache clean
