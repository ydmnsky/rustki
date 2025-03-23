# Просто личный cli-скриптик для запоминания сербских слов
Написан на rust, данные хранятся в SQLite DB
## Установка с ПМ карго
```
# билдим бинарник
git clone https://github.com/ydmnsky/rustki.git
cd rustki
cargo build --release
# кладем линк на бинарник в $PATH для удобства
sudo ln -s $(pwd)/target/release/rustki /usr/local/bin/rustki
```
## Использование:
Запустить тренажер
```rustki```
Добавить слово
```rustki add <слово на сербском> <перевод>```
Удалить слово
```rustki remove <слово на сербском>```
Очистить все данные
```rustki clear```
