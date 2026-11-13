#!/usr/bin/python3
import re
import sys
import json

exclusion_list = [
    "SubtreeMIBEntry",
    "MIBUpdater.start",
    "Failure in sendto"
]

def flatten_and_clean_log(input_file_path, output_file_path):
    # 1. Основное регулярное выражение для разбора структуры syslog
    # Группа 1: Внешняя дата (Nov  6 16:15:56.396963)
    # Группа 2: Имя хоста (tc-jet-lab-fab-swl01)
    # Группа 3: Уровень лога (INFO)
    # Группа 4: Процесс/сервис (bgp#supervisord: zebra)
    # Группа 5: Текст сообщения (все, что идет дальше)
    log_pattern = re.compile(
        r'^([A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2}\.\d{6})\s+(\S+)\s+([A-Z]+)\s+(\S+#\S+:*)\s+(.*)$'
    )

    exclude_pattern = "|".join(map(re.escape, exclusion_list))

    # 2. Регулярное выражение для поиска и удаления второй даты в формате YYYY/MM/DD HH:MM:SS
    nested_date_pattern = re.compile(r'\d{4}\/\d{2}\/\d{2}\s+\d{2}:\d{2}:\d{2}\s*')
    nested_bgp_pattern = re.compile(r'bgpd: BGP: ')
    nested_zebra1_pattern = re.compile(r'ZEBRA: ')
    nested_zebra2_pattern = re.compile(r'zebra: ')
    nested_zebra3_pattern = re.compile(r'zebra ')
    nested_zebra_pattern = re.compile(r'zebra: ZEBRA: ')
    nested_code_pattern = re.compile(r'\[[A-Z0-9]+-[A-Z0-9]+\]')
    nested_lldp_pattern = re.compile(r'\[lldp_syncd\]')
    nested_info_pattern = re.compile(r'INFO:')
    nested_ec_pattern = re.compile(r'\[EC\s\d+\]\s')

    try:
        with open(input_file_path, 'r', encoding='utf-8') as infile, \
             open(output_file_path, 'w', encoding='utf-8') as outfile:

            for line in infile:
                line = line.strip()
                if not line:
                    continue

                if re.search(exclude_pattern, line):
                    continue

                match = log_pattern.match(line)
                if match:
                    timestamp, host, log_level, component, message = match.groups()

                    # Удаляем вторую (вложенную) дату из сообщения
                    clean_message = nested_date_pattern.sub('', message)
                    clean_message = nested_bgp_pattern.sub('', clean_message)
                    clean_message = nested_zebra_pattern.sub('', clean_message)
                    clean_message = nested_zebra1_pattern.sub('', clean_message)
                    clean_message = nested_zebra2_pattern.sub('', clean_message)
                    clean_message = nested_code_pattern.sub('', clean_message)
                    clean_message = nested_lldp_pattern.sub('', clean_message)
                    clean_message = nested_info_pattern.sub('', clean_message)
                    clean_message = nested_zebra3_pattern.sub('', clean_message)
                    clean_message = nested_ec_pattern.sub('', clean_message)

                    # Убираем возможные двойные пробелы, которые могли остаться после удаления даты
                    clean_message = re.sub(r'\s+', ' ', clean_message).strip()
                    # Убираем все возможные двойные двоеточия из имени компонента
                    component = re.sub(r':', '', component).strip()
                    component = re.sub(r':', '', component).strip()
#                    component = component + ":"

                    # Форматируем в стандартную плоскую строку лога
                    standard_line = f"{timestamp} {host} {log_level} {component.strip()}: {clean_message}\n"

                    dict = {
                        "level": log_level,
                        "message": clean_message,
                        "timestamp": timestamp,
                        "host": host,
                        "component": component
                    }
                    json_string = json.dumps(dict, ensure_ascii=False) + "\n" # Преобразует словарь в JSON-строку

#                    outfile.write(standard_line)
                    outfile.write(json_string)
#                else:
                    # Если строка не стандартная (например, многострочный стек-трейс),
                    # записываем её как есть, чтобы не потерять информацию
#                    outfile.write(f"{line}\n")
        print(f"Лог успешно очищен и сохранен в: {output_file_path}")

    except FileNotFoundError:
        print(f"Ошибка: Файл '{input_file_path}' не найден.")
    except Exception as e:
        print(f"Произошла ошибка при обработке: {e}")

if __name__ == "__main__":
    # Аргументы командной строки по умолчанию: input.log -> clean_output.log
    in_file = sys.argv[1] if len(sys.argv) > 1 else 'input.log'
    out_file = sys.argv[2] if len(sys.argv) > 2 else 'clean_output.log'
    flatten_and_clean_log(in_file, out_file)
#    flatten_and_clean_log("/home/asha/logllms/new/logai/log-2-analyze/log_swl01.log", "/home/asha/logllms/new/logai/log-2-analyze/log_swl01_flatten.log")
