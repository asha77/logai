#!/bin/bash

# Проверяем, передан ли аргумент (имя архива)
if [ -z "$1" ]; then
    echo "Ошибка: Не указан файл архива."
    echo "Использование: $0 <имя_главного_архива.tar.gz>"
    exit 1
fi

MAIN_ARCHIVE="$1"

echo "Получен параметр: $MAIN_ARCHIVE"


# Проверяем существование файла
if [ ! -f "$MAIN_ARCHIVE" ]; then
    echo "Ошибка: Файл '$MAIN_ARCHIVE' не найден."
    exit 1
fi


# Инициализируем переменную, которая найдет реальную папку c архивом
SCRIPT_DIR="$(cd "$(dirname "$(readlink -f "$MAIN_ARCHIVE")")" && pwd)"

echo "Директория: $SCRIPT_DIR"


# Теперь используем абсолютный путь к архиву
MAIN_ARCHIVE="$SCRIPT_DIR/$MAIN_ARCHIVE"

echo "Абсолютный полный путь: $MAIN_ARCHIVE"

# Проверяем, существует ли архив (для надежности)
if [ ! -f "$MAIN_ARCHIVE" ]; then
    echo "Ошибка: Архив не найден по пути $MAIN_ARCHIVE"
    exit 1
fi


# Получаем абсолютный путь к архиву, чтобы не потерять его при переходе в папки
MAIN_ARCHIVE_PATH=$(realpath "$MAIN_ARCHIVE")

# Создаем уникальную временную папку
TMP_DIR=$(mktemp -d -t syslog_unpack_XXXXXX)
echo "=== Распаковка главного архива во временную папку $TMP_DIR ==="

# Шаг 1: Распаковываем главный архив во временную директорию
if ! tar -xzf "$MAIN_ARCHIVE" -C "$TMP_DIR" --wildcards --strip-components=2 '*/log/'; then
    echo "Ошибка при распаковке главного архива."
    rm -rf "$TMP_DIR"
    exit 1
fi

# Переходим во временную папку
cd "$TMP_DIR" || exit 1

# Имя итогового файла (создастся в исходной папке, откуда запускали скрипт)
OUTPUT_FILE="${MAIN_ARCHIVE_PATH%.tar.gz}.txt"

# Выделяем путь к папке
dir_path=$(dirname "$OUTPUT_FILE")

# Выделяем только имя файла
file_name_only=$(basename "$OUTPUT_FILE")

# Извлекаем нужную часть (теперь cut отработает правильно, так как путей нет)
core_name=$(echo "$file_name_only" | cut -d'_' -f3)

# Собираем всё вместе: путь + префикс + очищенное имя + расширение .txt
OUTPUT_FILE="${SCRIPT_DIR}/syslog_${core_name}.txt"

echo "=== Сборка логов в хронологическом порядке -> $OUTPUT_FILE ==="

# Очищаем или создаем файл вывода
> "$OUTPUT_FILE"

# Шаг 2: Находим и обрабатываем сжатые архивы syslog.*.tar.gz
# Сортируем их по версии/номеру (sort -V) и переворачиваем (-r),
# чтобы сначала шли старые логи (например, syslog.4.tar.gz), а затем свежие (syslog.2.tar.gz)
mapfile -t COMPRESSED_LOGS < <(ls -1 syslog.*.gz 2>/dev/null | sort -V -r)

echo "${COMPRESSED_LOGS[@]}"

for archive in "${COMPRESSED_LOGS[@]}"; do
    if [ -f "$archive" ]; then
        echo "Распаковка вложенного архива: $archive"
        # zcat извлекает содержимое файлов архива прямо в стандартный вывод (stdout)
        zcat "$archive" >> "$OUTPUT_FILE"
    fi
done

# Шаг 3: Добавляем свежие несжатые логи (если они присутствуют)
# Обычно это syslog.gz (текущий активный)
if [ -f "syslog.gz" ]; then
    echo "Добавление актуального лога: syslog"
    zcat "syslog.gz" >> "$OUTPUT_FILE"
fi

# Шаг 4: Очистка временных файлов
echo "=== Очистка временных файлов ==="
cd - > /dev/null || exit
rm -rf "$TMP_DIR"

echo "Готово! Все логи успешно объединены в файл: $OUTPUT_FILE"
