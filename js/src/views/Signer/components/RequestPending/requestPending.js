// Copyright 2015, 2016 Ethcore (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

import React, { Component, PropTypes } from 'react';

import TransactionPending from '../TransactionPending';
import SignRequest from '../SignRequest';

export default class RequestPending extends Component {
  static propTypes = {
    id: PropTypes.object.isRequired,
    onConfirm: PropTypes.func.isRequired,
    onReject: PropTypes.func.isRequired,
    isSending: PropTypes.bool.isRequired,
    date: PropTypes.instanceOf(Date).isRequired,
    payload: PropTypes.oneOfType([
      PropTypes.shape({ transaction: PropTypes.object.isRequired }),
      PropTypes.shape({ sign: PropTypes.object.isRequired })
    ]).isRequired,
    className: PropTypes.string,
    isTest: PropTypes.bool.isRequired,
    store: PropTypes.object.isRequired
  };

  onConfirm = data => {
    const { onConfirm, payload } = this.props;

    data.payload = payload;
    onConfirm(data);
  };

  render () {
    const { payload, id, className, isSending, date, onReject, isTest, store } = this.props;

    if (payload.sign) {
      const { sign } = payload;

      return (
        <SignRequest
          className={ className }
          onConfirm={ this.onConfirm }
          onReject={ onReject }
          isSending={ isSending }
          isFinished={ false }
          id={ id }
          address={ sign.address }
          hash={ sign.hash }
          isTest={ isTest }
          store={ store }
          />
      );
    }

    if (payload.transaction) {
      const { transaction } = payload;

      return (
        <TransactionPending
          className={ className }
          onConfirm={ this.onConfirm }
          onReject={ onReject }
          isSending={ isSending }
          id={ id }
          gasPrice={ transaction.gasPrice }
          gas={ transaction.gas }
          data={ transaction.data }
          from={ transaction.from }
          to={ transaction.to }
          value={ transaction.value }
          date={ date }
          isTest={ isTest }
          store={ store }
          />
      );
    }

    // Unknown payload
    return null;
  }
}
