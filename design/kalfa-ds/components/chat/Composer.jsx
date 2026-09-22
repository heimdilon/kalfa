import React from 'react';
import {Textarea} from '../forms/Textarea.jsx';
import {Select} from '../forms/Select.jsx';
import {IconButton} from '../core/IconButton.jsx';

export function Composer({placeholder='Bir şey sorun veya bir görev tanımlayın…',value,onChange,
  model='glm-5.3',models=['glm-5.3','deepseek-v4-flash','qwen3.8-flash-next','auto'],onModelChange,
  mode='Sıkı',modes=['İnceleme','Sıkı','Proje','Serbest'],onModeChange,onSend,disabled,style}){
  return (
    <Textarea accent rows={2} placeholder={placeholder} value={value} disabled={disabled}
      onChange={onChange} style={style}
      footer={
        <>
          <Select size="sm" icon="box" tone="accent" value={model} options={models} onChange={onModelChange}/>
          <Select size="sm" icon="shield" value={mode} options={modes} onChange={onModeChange}/>
          <span style={{flex:1}}/>
          <IconButton icon="paperclip" label="Dosya ekle" size="sm"/>
          <IconButton icon="at-sign" label="Bağlam ekle" size="sm"/>
          <IconButton icon="code" label="Kod bloğu" size="sm"/>
          <IconButton icon="send" label="Gönder" variant="primary" onClick={onSend}/>
        </>
      }/>
  );
}
