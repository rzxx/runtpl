Отчет по проекту {{ project }}:
{{foreach team in teams}}
Команда: {{ team.name }}
  Участники:
    {{foreach member in team.members}}
    - {{ member }}
    {{endfor}}
{{endfor}}